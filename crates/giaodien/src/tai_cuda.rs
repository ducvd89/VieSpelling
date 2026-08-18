//! Tải các DLL runtime của CUDA từ NVIDIA về, ngay trong ứng dụng.
//!
//! # Vì sao ứng dụng phải tự tải thay vì đóng gói kèm
//!
//! Ba DLL ấy nặng **493 MB** — `cublasLt64_13.dll` một mình 442 MB. Gói kèm thì
//! bản cài gần 600 MB, mà phần lớn người dùng tải về sẽ không bao giờ dùng tới
//! chúng: không có card NVIDIA thì mô hình không chạy được, và bộ dò vẫn làm việc
//! đầy đủ bằng các tầng luật.
//!
//! Nên bản cài chỉ có hai file thực thi, còn phần nặng thì tải khi nào cần. Điều
//! kiện để làm được thế là **nạp trễ** — xem `nap_tre_cublas` trong `build.rs`.
//! Không có nó thì Windows đòi DLL ngay lúc mở file exe và ứng dụng không có chỗ
//! nào để hỏi han gì.
//!
//! # Lấy ở đâu
//!
//! Kho `redist` của NVIDIA — chính chỗ mà mọi bản CUDA Toolkit lấy ra, và mấy DLL
//! này nằm trong danh sách được phép phát hành lại. Địa chỉ gán cứng theo phiên
//! bản; muốn nâng thì đọc `redistrib_<bản>.json` ở cùng thư mục để lấy đường dẫn
//! và mã băm mới.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const GOC: &str = "https://developer.download.nvidia.com/compute/cuda/redist";

/// Một gói cần tải: (đường dẫn trong kho, cỡ thật, mô tả cho người dùng).
///
/// Cỡ ghi sẵn để vẽ được thanh tiến độ **tổng** ngay từ đầu, thay vì phải chờ
/// máy chủ trả `Content-Length` của gói thứ hai mới biết còn bao xa.
const GOI: [(&str, u64, &str); 2] = [
    (
        "libcublas/windows-x86_64/libcublas-windows-x86_64-13.5.1.27-archive.zip",
        391_186_432,
        "thư viện đại số cuBLAS",
    ),
    (
        "cuda_cudart/windows-x86_64/cuda_cudart-windows-x86_64-13.3.29-archive.zip",
        2_621_440,
        "runtime CUDA",
    ),
];

/// Tên DLL cần moi ra khỏi mấy gói ấy. Trong zip chúng nằm sâu trong `bin/`.
const CAN_LAY: [&str; 3] = ["cublas64_", "cublasLt64_", "cudart64_"];

#[derive(Default)]
pub struct TienDo {
    /// Số byte đã tải, cộng dồn qua cả hai gói.
    pub da_tai: u64,
    /// Tổng số byte phải tải.
    pub tong: u64,
    /// Byte mỗi giây, tính trên khoảng gần đây chứ không phải trung bình cả lượt
    /// — trung bình cả lượt thì lúc mạng tụt vẫn hiện con số đẹp của mấy phút
    /// trước, và người dùng không biết là nó đang đứng.
    pub toc_do: f64,
    pub mo_ta: String,
    pub xong: bool,
    pub loi: Option<String>,
    /// Người dùng bấm huỷ. Luồng tải xem cờ này giữa các khối đọc.
    pub huy: bool,
}

impl TienDo {
    pub fn ty_le(&self) -> f32 {
        if self.tong == 0 {
            0.0
        } else {
            (self.da_tai as f64 / self.tong as f64) as f32
        }
    }

    /// Còn bao lâu nữa, tính bằng giây. `None` khi chưa đủ dữ liệu để đoán.
    pub fn con_lai(&self) -> Option<f64> {
        (self.toc_do > 1024.0 && self.tong > self.da_tai)
            .then(|| (self.tong - self.da_tai) as f64 / self.toc_do)
    }
}

/// Bắt đầu tải ở luồng nền. Trả về chỗ để giao diện đọc tiến độ.
pub fn bat_dau(dich: PathBuf) -> Arc<Mutex<TienDo>> {
    let td = Arc::new(Mutex::new(TienDo {
        tong: GOI.iter().map(|g| g.1).sum(),
        mo_ta: "đang nối tới NVIDIA…".into(),
        ..Default::default()
    }));
    let ra = Arc::clone(&td);
    std::thread::spawn(move || {
        if let Err(e) = chay(&dich, &ra) {
            let mut t = ra.lock().unwrap();
            t.loi = Some(format!("{e:#}"));
            t.xong = true;
        } else {
            ra.lock().unwrap().xong = true;
        }
    });
    td
}

fn chay(dich: &Path, td: &Arc<Mutex<TienDo>>) -> anyhow::Result<()> {
    std::fs::create_dir_all(dich)?;
    let mut da_xong = 0u64;
    for (duong, co, ten) in GOI {
        {
            let mut t = td.lock().unwrap();
            t.mo_ta = format!("đang tải {ten}…");
        }
        let byte = tai_mot(&format!("{GOC}/{duong}"), da_xong, td)?;
        da_xong += co;
        {
            let mut t = td.lock().unwrap();
            t.mo_ta = format!("đang mở gói {ten}…");
            // Gói tải xong có thể lệch cỡ ghi sẵn đôi chút; kéo thanh về đúng mốc
            // để nó không tụt lại khi sang gói sau.
            t.da_tai = da_xong;
        }
        moi_dll(&byte, dich)?;
    }
    let mut t = td.lock().unwrap();
    t.mo_ta = "xong".into();
    Ok(())
}

/// Tải một gói vào bộ nhớ, cập nhật tiến độ theo từng khối.
///
/// Giữ trong RAM chứ không ghi ra file tạm: gói lớn nhất 373 MB, mà máy chạy nổi
/// mô hình 9 tỷ tham số thì thừa chỗ. Đổi lại không phải dọn file tạm khi người
/// dùng huỷ giữa chừng.
fn tai_mot(url: &str, da_xong: u64, td: &Arc<Mutex<TienDo>>) -> anyhow::Result<Vec<u8>> {
    let tra_loi = ureq::get(url).call().map_err(|e| anyhow::anyhow!("không tải được: {e}"))?;
    let mut doc = tra_loi.into_reader();
    let mut ra: Vec<u8> = Vec::new();
    let mut dem = [0u8; 64 * 1024];
    // Mốc để tính tốc độ: mỗi lần chốt lại thì lấy số byte và thời điểm mới.
    let mut moc = Instant::now();
    let mut moc_byte = 0u64;
    loop {
        let n = doc.read(&mut dem)?;
        if n == 0 {
            break;
        }
        ra.extend_from_slice(&dem[..n]);
        let giay = moc.elapsed().as_secs_f64();
        if giay >= 0.4 {
            let mut t = td.lock().unwrap();
            t.toc_do = (ra.len() as u64 - moc_byte) as f64 / giay;
            t.da_tai = da_xong + ra.len() as u64;
            if t.huy {
                anyhow::bail!("đã huỷ");
            }
            moc = Instant::now();
            moc_byte = ra.len() as u64;
        }
    }
    Ok(ra)
}

/// Moi ba DLL ra khỏi gói zip, bỏ qua phần còn lại.
///
/// Gói `libcublas` có cả thư viện tĩnh và header — cộng lại hơn một GB sau khi
/// giải nén. Chỉ lấy đúng thứ cần thì thư mục cài không phình lên vô ích.
fn moi_dll(byte: &[u8], dich: &Path) -> anyhow::Result<()> {
    let mut kho = zip::ZipArchive::new(std::io::Cursor::new(byte))?;
    for i in 0..kho.len() {
        let mut muc = kho.by_index(i)?;
        let Some(ten) = muc.enclosed_name().and_then(|p| p.file_name().map(|x| x.to_owned()))
        else {
            continue;
        };
        let ten = ten.to_string_lossy().to_string();
        if !CAN_LAY.iter().any(|t| ten.starts_with(t)) || !ten.ends_with(".dll") {
            continue;
        }
        let mut noi_dung = Vec::new();
        muc.read_to_end(&mut noi_dung)?;
        std::fs::write(dich.join(&ten), noi_dung)?;
    }
    Ok(())
}
