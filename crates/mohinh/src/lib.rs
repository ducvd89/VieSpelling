//! Chấm điểm câu bằng một mô hình ngôn ngữ GGUF chạy trên máy.
//!
//! # Vì sao **chấm điểm** chứ không **hỏi** mô hình
//!
//! Cách hiển nhiên là đưa câu cho mô hình rồi bảo "sửa lỗi chính tả giúp". Cách
//! ấy sai ở đây, và sai theo kiểu không vá được:
//!
//! - Mô hình **viết lại văn của tác giả**. Bảo nó sửa chính tả thì nó tiện tay
//!   đổi luôn từ ngữ, gộp câu, bỏ chữ nó thấy thừa. Trong một cuốn tiểu thuyết
//!   thì đó không phải sửa lỗi, đó là hỏng sách.
//! - Mô hình nhỏ **bịa**. Gặp tên riêng lạ nó đổi thành tên nó biết.
//! - Kết quả **không lặp lại được**, nên không kiểm thử được.
//!
//! Ở đây mô hình không được sinh ra chữ nào. Nó chỉ làm đúng một việc: cho một
//! câu, trả về log-xác suất trung bình mỗi token. Các cách sửa thì do tầng luật
//! và tầng cấu tạo âm tiết sinh ra — chúng bảo đảm mọi ứng viên đều là tiếng
//! Việt viết đúng — còn mô hình chỉ **xếp hạng** chúng. Chuỗi mô hình chưa từng
//! thấy thì không cách nào lọt vào sách được, vì nó không tồn tại trong danh
//! sách ứng viên.
//!
//! # Chọn mô hình
//!
//! Cỡ nào cũng chạy được, nhưng cỡ có ảnh hưởng thật, và ảnh hưởng **không đều**
//! giữa hai việc mà tầng này làm:
//!
//! - Chọn giữa `thuơng` và `thương` thì mô hình nào cũng làm được — một bên là
//!   chuỗi ký tự mô hình chưa từng thấy, chênh lệch điểm rất lớn.
//! - Chọn giữa `để dành` và `để giành`, `chia sẻ` và `chia xẻ` thì phải **hiểu
//!   câu**. Cả hai đều là tiếng Việt trôi chảy, chênh lệch điểm nhỏ, và mô hình
//!   nhỏ hay chấm sai hướng — mà ngưỡng an toàn ở [`TuyChon::nguong_mo_hinh`]
//!   thì biến "chấm sai hướng" thành "không sửa gì", tức là mất luôn tác dụng.
//!
//! Nên mô hình lớn đáng giá đúng ở lớp lỗi thứ hai. Bản dựng mặc định bật CUDA
//! và đẩy toàn bộ mô hình sang GPU (xem [`SO_LOP_GPU`]), nên một mô hình 9 tỷ
//! tham số lượng tử Q4 chạy được thoải mái trên card 16 GB.
//!
//! [`TuyChon::nguong_mo_hinh`]: chinhta::soat::TuyChon::nguong_mo_hinh

use anyhow::{Context, Result};
use chinhta::soat::ChamDiem;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use std::cell::RefCell;
use std::num::NonZeroU32;
use std::path::Path;

/// Số token tối đa cho một lượt chấm. Câu dài hơn bị cắt bớt đầu.
const NGU_CANH: u32 = 512;

/// Số lớp đẩy sang GPU. Đặt cao hơn số lớp của mọi mô hình để nó **đẩy hết**;
/// llama.cpp tự cắt xuống đúng số lớp thật có.
///
/// Vì sao đẩy hết chứ không chia đôi: chỉ cần một lớp nằm lại CPU là mỗi lượt
/// chấm phải chuyển trạng thái qua lại qua khe PCIe, và với hàng nghìn lượt câu
/// ngắn thì phần chuyển ấy át hẳn phần tính.
const SO_LOP_GPU: u32 = 999;

/// Ngưỡng VRAM tối thiểu để nhận là card dùng được.
///
/// Dưới mức này thì mô hình không nằm trọn trong VRAM, llama.cpp phải để một
/// phần lớp lại CPU, và ta rơi đúng vào cái đã bị cấm — chỉ có điều rơi vào một
/// cách âm thầm. Thà báo lỗi.
const VRAM_TOI_THIEU: usize = 4 * 1024 * 1024 * 1024;

/// Card đồ hoạ dùng được, nếu có.
///
/// Chỉ nhận GPU rời (`Gpu`). GPU tích hợp bị loại: nó dùng chung RAM với hệ
/// thống nên không nhanh hơn CPU là bao, mà lại làm ta tưởng đang chạy trên card.
pub fn card_dung_duoc() -> Option<llama_cpp_2::LlamaBackendDevice> {
    llama_cpp_2::list_llama_ggml_backend_devices()
        .into_iter()
        .filter(|d| d.device_type == llama_cpp_2::LlamaBackendDeviceType::Gpu)
        .max_by_key(|d| d.memory_total)
}

/// Mô tả mọi thiết bị llama.cpp nhìn thấy — để in ra khi báo lỗi thiếu card.
pub fn liet_ke_thiet_bi() -> String {
    let d = llama_cpp_2::list_llama_ggml_backend_devices();
    if d.is_empty() {
        return "(không thấy thiết bị nào)".into();
    }
    d.iter()
        .map(|x| {
            format!(
                "{} [{}] {:?} {:.1} GB",
                x.description,
                x.backend,
                x.device_type,
                x.memory_total as f64 / 1e9
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// llama.cpp chỉ cho khởi động nền **một lần cho cả tiến trình** — gọi
/// `LlamaBackend::init()` lần thứ hai là lỗi. Người dùng đổi mô hình giữa chừng
/// là chuyện thường, nên giữ nền ở đây thay vì trong từng [`MoHinh`].
static NEN: std::sync::OnceLock<Result<LlamaBackend, String>> = std::sync::OnceLock::new();

fn nen() -> Result<&'static LlamaBackend> {
    match NEN.get_or_init(|| LlamaBackend::init().map_err(|e| e.to_string())) {
        Ok(n) => Ok(n),
        Err(e) => anyhow::bail!("không khởi động được llama.cpp: {e}"),
    }
}

pub struct MoHinh {
    /// **Thứ tự khai báo trường ở đây là thứ tự thả.** `ngu_canh` mượn
    /// `mo_hinh`, nên nó phải đứng trước để bị thả trước; đảo lại là dùng bộ
    /// nhớ đã giải phóng lúc thoát ứng dụng.
    ngu_canh: RefCell<Option<llama_cpp_2::context::LlamaContext<'static>>>,
    /// Trong `Box` nên địa chỉ cố định — đó là điều kiện để mượn `'static` ở
    /// dưới là đúng.
    mo_hinh: Box<LlamaModel>,
    /// Đếm số lượt, chỉ để hiện trong báo cáo.
    pub so_luot: std::cell::Cell<u64>,
    pub duong_dan: std::path::PathBuf,
}

impl MoHinh {
    /// Nạp mô hình. **Không có card đồ hoạ rời thì trả lỗi, không chạy bằng CPU.**
    ///
    /// Lùi về CPU nghe thì tử tế nhưng ở đây là cái bẫy: mô hình 9 tỷ tham số
    /// chấm trên CPU chậm hơn hàng chục lần, và người dùng bấm "Kiểm và sửa"
    /// rồi ngồi nhìn thanh tiến trình hàng giờ mà không hiểu vì sao — chương
    /// trình có báo gì đâu, nó vẫn đang chạy. Báo lỗi ngay thì người ta biết
    /// đường xử lý: cài driver, hoặc bỏ mô hình đi mà chạy bằng luật.
    pub fn nap(duong_dan: &Path) -> Result<MoHinh> {
        Self::nap_co_tien_do(duong_dan, |_| {})
    }

    /// `tien_do` nhận tỷ lệ 0…1 trong lúc đọc file mô hình.
    ///
    /// Nạp 5 GB từ đĩa sang VRAM mất vài giây tới vài chục giây tuỳ ổ, và trong
    /// suốt quãng ấy chương trình không làm gì khác. Không báo ra thì đó là
    /// quãng đứng hình dài nhất của cả lượt chạy.
    pub fn nap_co_tien_do(
        duong_dan: &Path,
        mut tien_do: impl FnMut(f32) + 'static,
    ) -> Result<MoHinh> {
        let nen = nen()?;

        if !cfg!(feature = "cuda") {
            anyhow::bail!(
                "bản dựng này không có CUDA nên mô hình chỉ chạy được bằng CPU. \
                 Dựng lại với tính năng `cuda`, hoặc bỏ mô hình để chạy bằng luật."
            );
        }
        let card = card_dung_duoc().ok_or_else(|| {
            anyhow::anyhow!(
                "không thấy card đồ hoạ rời nào. llama.cpp chỉ nhìn thấy: {}. \
                 Kiểm lại driver NVIDIA, hoặc bỏ mô hình để chạy bằng luật.",
                liet_ke_thiet_bi()
            )
        })?;
        if card.memory_total < VRAM_TOI_THIEU {
            anyhow::bail!(
                "card {} chỉ có {:.1} GB VRAM, không đủ để giữ trọn mô hình — \
                 phần tràn sẽ chạy bằng CPU. Dùng mô hình nhỏ hơn, hoặc bỏ mô hình.",
                card.description,
                card.memory_total as f64 / 1e9
            );
        }

        let tham_so = LlamaModelParams::default()
            .with_n_gpu_layers(SO_LOP_GPU)
            // Trả `true` nghĩa là "chạy tiếp"; trả `false` là huỷ giữa chừng.
            .with_progress_callback(move |t| {
                tien_do(t);
                true
            });
        let mo_hinh = Box::new(
            LlamaModel::load_from_file(nen, duong_dan, &tham_so)
                .with_context(|| format!("không nạp được mô hình {}", duong_dan.display()))?,
        );

        // Số luồng chỉ ăn thua ở đường lùi về CPU, nhưng ở đó nó ăn thua rất
        // lớn: llama.cpp mặc định **4 luồng** bất kể máy có bao nhiêu nhân.
        let so_luong = std::thread::available_parallelism()
            .map(|n| (n.get() / 2).clamp(4, 32))
            .unwrap_or(4) as i32;
        let tham_so_nc = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(NGU_CANH))
            .with_n_batch(NGU_CANH)
            .with_n_threads(so_luong)
            .with_n_threads_batch(so_luong);

        // `LlamaContext` mượn `LlamaModel` mà cả hai phải nằm chung một struct.
        // Rust không diễn đạt được kiểu tự tham chiếu ấy, nên kéo dài đời mượn
        // bằng tay. Đúng vì hai điều kiện: `mo_hinh` nằm trong `Box` nên không
        // dời chỗ khi struct bị di chuyển, và `ngu_canh` khai trước nên bị thả
        // trước.
        let muon: &'static LlamaModel = unsafe { &*(&*mo_hinh as *const LlamaModel) };
        let nc = muon.new_context(nen, tham_so_nc).context("không tạo được ngữ cảnh")?;

        Ok(MoHinh {
            ngu_canh: RefCell::new(Some(nc)),
            mo_hinh,
            so_luot: std::cell::Cell::new(0),
            duong_dan: duong_dan.to_path_buf(),
        })
    }

    /// Mô tả ngắn để hiện trong báo cáo.
    pub fn mo_ta(&self) -> String {
        let ty = self.mo_hinh.n_params() as f64 / 1e9;
        let ten = self
            .duong_dan
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        format!("{ten} ({ty:.2} tỷ tham số)")
    }

    /// Log-xác suất của câu, **chia cho số ký tự**.
    ///
    /// Phải chia cho cái gì đó: lấy tổng thì chuỗi dài luôn thua chuỗi ngắn,
    /// tức là mọi phép xoá chữ đều được thưởng.
    ///
    /// Nhưng chia cho **số token** thì hỏng theo kiểu khó thấy: các ứng viên
    /// tách ra số token khác nhau, và thêm một token dễ đoán có thể **nâng**
    /// điểm trung bình lên. Đo được: mô hình chấm `Hắn phảii đi ngay.` cao hơn
    /// `Hắn phải đi ngay.` 0,167 — câu sai thắng câu đúng, vì `phảii` tách thành
    /// `phải` + `i` và cái đuôi `i` ấy không đắt bằng mức nó kéo mẫu số lên.
    ///
    /// Số ký tự thì không phụ thuộc vào cách tách token, mà các ứng viên ở đây
    /// chỉ chênh nhau một hai ký tự nên mẫu số gần như không đổi — đúng cái ta
    /// cần khi so hai câu gần giống hệt nhau.
    fn cham_that(&self, cau: &str) -> Result<f32> {
        // `AddBos::Always` đọc dễ nhầm: nó **không** ép thêm BOS, nó bật cờ
        // `add_special` của llama.cpp, tức là "thêm token đặc biệt theo đúng
        // khai báo của mô hình". Mô hình nào khai không dùng BOS — Qwen chẳng
        // hạn — thì không có gì được thêm. Đổi sang `Never` mới là sai, vì lúc
        // ấy mô hình nào *có* dùng BOS sẽ bị chấm trong một ngữ cảnh nó chưa
        // từng gặp lúc huấn luyện.
        let mut token = self
            .mo_hinh
            .str_to_token(cau, AddBos::Always)
            .context("không tách được token")?;
        if token.len() < 2 {
            return Ok(0.0);
        }
        if token.len() > NGU_CANH as usize {
            // Cắt phần đầu, giữ phần đuôi — chỗ sửa gần cuối cửa sổ hơn.
            token.drain(..token.len() - NGU_CANH as usize);
        }
        let n = token.len();

        let mut lo = LlamaBatch::new(n, 1);
        for (i, &t) in token.iter().enumerate() {
            // Cần logits ở **mọi** vị trí: xác suất của token i lấy từ phân bố
            // dự đoán ở vị trí i-1, nên phải giữ lại hết chứ không chỉ vị trí
            // cuối như lúc sinh chữ.
            lo.add(t, i as i32, &[0], true)?;
        }
        let mut muon = self.ngu_canh.borrow_mut();
        let nc = muon.as_mut().context("ngữ cảnh đã bị thả")?;
        // Xoá bộ nhớ đệm KV trước mỗi lượt: các lượt chấm **không** nối tiếp
        // nhau, mỗi câu là một chuỗi độc lập. Quên bước này thì câu sau được
        // chấm như thể nó viết tiếp câu trước, và điểm số thành vô nghĩa.
        nc.clear_kv_cache();
        nc.decode(&mut lo).context("mô hình không chạy được")?;

        let mut tong = 0.0f32;
        for i in 1..n {
            let logits = nc.get_logits_ith(i as i32 - 1);
            tong += log_xac_suat(logits, token[i].0 as usize);
        }
        self.so_luot.set(self.so_luot.get() + 1);
        let so_ky_tu = cau.chars().count().max(1) as f32;
        Ok(tong / so_ky_tu)
    }
}

/// log softmax tại một chỉ số, tính ổn định về mặt số học.
///
/// Trừ đi giá trị lớn nhất trước khi lấy `exp`: logits của mô hình thường nằm
/// quanh 20–30, mà `exp(30)` đã tràn khỏi khoảng an toàn của f32 khi cộng dồn
/// hàng vạn số hạng.
fn log_xac_suat(logits: &[f32], chi_so: usize) -> f32 {
    let Some(&muc_tieu) = logits.get(chi_so) else {
        return -20.0;
    };
    let lon_nhat = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let tong: f32 = logits.iter().map(|&x| (x - lon_nhat).exp()).sum();
    muc_tieu - lon_nhat - tong.ln()
}

impl ChamDiem for MoHinh {
    fn cham(&self, cau: &str) -> f32 {
        // Lỗi lúc chấm không được làm hỏng cả lượt xử lý sách. Trả điểm rất
        // thấp thì ứng viên ấy tự thua, và bản gốc — cũng chấm bằng chính hàm
        // này — thắng. Nói cách khác mô hình hỏng thì ứng dụng thoái về chế độ
        // không mô hình, chứ không sửa bừa.
        self.cham_that(cau).unwrap_or(f32::NEG_INFINITY)
    }
}

/// Kiểm nhanh xem file có phải GGUF không, để báo lỗi tử tế thay vì để
/// llama.cpp in ra một trang chữ đỏ.
pub fn la_gguf(duong_dan: &Path) -> bool {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(duong_dan) else {
        return false;
    };
    let mut dau = [0u8; 4];
    f.read_exact(&mut dau).is_ok() && &dau == b"GGUF"
}

#[cfg(test)]
mod kiem {
    use super::*;

    #[test]
    fn log_softmax_khong_tran_so() {
        // Logits thật của mô hình hay nằm quanh 20–30. Tính thẳng `exp` rồi
        // cộng là tràn, mà tràn thì ra `inf` và mọi ứng viên bằng điểm nhau —
        // hỏng lặng lẽ, không lỗi nào bật ra.
        let l = vec![30.0f32, 29.0, 28.0, 1.0];
        let p = log_xac_suat(&l, 0);
        assert!(p.is_finite(), "tràn số");
        assert!(p < 0.0 && p > -3.0, "log-xác suất vô lý: {p}");
        // Tổng xác suất phải bằng 1.
        let tong: f32 = (0..l.len()).map(|i| log_xac_suat(&l, i).exp()).sum();
        assert!((tong - 1.0).abs() < 1e-4, "tổng xác suất {tong}");
    }

    #[test]
    fn chi_so_ngoai_bang_khong_no() {
        assert_eq!(log_xac_suat(&[1.0, 2.0], 99), -20.0);
    }
}
