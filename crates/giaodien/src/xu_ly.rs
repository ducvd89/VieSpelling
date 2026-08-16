//! Chạy toàn bộ đường đi trên một cuốn EPUB.
//!
//! Đi **hai lượt** qua sách, và thứ tự ấy không đảo được. Lượt một chỉ đếm kiểu
//! đặt dấu thanh (`hòa` hay `hoà`) trên toàn bộ sách; lượt hai mới sửa. Lý do
//! là kiểu đặt dấu không có bên nào sai — chỉ có chuyện cả sách nên nhất quán —
//! nên phải đọc hết sách mới biết kéo về phe nào. Sửa ngay từ đoạn đầu là áp
//! lựa chọn của mình lên sách người ta, và với sách viết theo kiểu thiểu số thì
//! đó là hàng nghìn thay đổi sai hướng.
//!
//! Bản gốc **không bao giờ bị ghi đè**: kết quả luôn ra một file khác.

use anyhow::{Context, Result};
use crate::nhat_ky::Bao;
use chinhta::dau_thanh::{self, DemKieu, Kieu};
use chinhta::doi_chieu;
use chinhta::soat::{BoSoat, ChamDiem, ChoXet, TuyChon};
use chinhta::sua::{Loai, SuaDoi};
use sach::{quet, Epub};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Default)]
pub struct KetQuaSach {
    pub nhan_de: Option<String>,
    pub so_file: usize,
    pub so_doan: usize,
    pub so_chu: usize,
    /// Mọi phép sửa đã áp, để viết báo cáo.
    pub da_sua: Vec<SuaDoi>,
    /// Những chỗ ngờ mà không đủ căn cứ để tự sửa.
    pub chua_sua: Vec<ChoXet>,
    /// Số chỗ tìm ra nhưng **không vá được** vì nằm vắt qua ranh giới thẻ HTML.
    pub vuong_the: usize,
    pub kieu_dau: Kieu,
    pub dem_kieu: DemKieu,
    pub co_mo_hinh: bool,
}

impl KetQuaSach {
    /// Đếm số phép sửa theo loại, để hiện bảng tổng kết.
    pub fn dem_theo_loai(&self) -> BTreeMap<&'static str, usize> {
        let mut m = BTreeMap::new();
        for s in &self.da_sua {
            *m.entry(s.loai.ten()).or_insert(0) += 1;
        }
        m
    }
}

/// Xử lý một cuốn sách.
///
/// Mọi thứ đi ra ngoài qua [`Bao`] — thanh tiến trình lẫn nhật ký. Chạy trên
/// luồng nền nên hàm này không được đụng vào giao diện.
pub fn xu_ly(
    dau_vao: &Path,
    dau_ra: &Path,
    tuy_chon: TuyChon,
    mo_hinh: Option<&dyn ChamDiem>,
    bao: &mut Bao,
) -> Result<KetQuaSach> {
    bao.buoc(format!("Mở {}", dau_vao.file_name().unwrap_or_default().to_string_lossy()));
    let mut epub = Epub::nap(dau_vao)?;
    bao.chi_tiet(format!("{} mục trong file zip", epub.muc.len()));

    let chi_so = epub.chi_so_van_ban();
    if chi_so.is_empty() {
        anyhow::bail!("không tìm thấy file nội dung nào trong EPUB");
    }
    let nhan_de = epub.nhan_de();
    bao.buoc(format!(
        "«{}» — {} file nội dung",
        nhan_de.as_deref().unwrap_or("không rõ nhan đề"),
        chi_so.len()
    ));

    let mut kq = KetQuaSach {
        nhan_de,
        so_file: chi_so.len(),
        co_mo_hinh: mo_hinh.is_some(),
        ..Default::default()
    };

    // ---- Lượt 1: đếm kiểu đặt dấu thanh trên cả sách ----
    bao.buoc("Lượt 1/2 — đọc cả sách để đếm kiểu đặt dấu thanh");
    let mot_phan = (chi_so.len() / 20).max(1);
    let mut dem = DemKieu::default();
    let mut so_doan_doc = 0usize;
    for (k, &i) in chi_so.iter().enumerate() {
        if k % mot_phan == 0 {
            bao.tien_do(
                0.15 * k as f32 / chi_so.len() as f32,
                format!("đọc sách… {k}/{} file", chi_so.len()),
            );
        }
        let Some(noi_dung) = sach::doc_chuoi(&epub.muc[i].noi_dung) else {
            bao.canh_bao(format!("{} không phải UTF-8 hợp lệ — bỏ qua", epub.muc[i].ten));
            continue;
        };
        for d in quet::quet(&noi_dung) {
            so_doan_doc += 1;
            dau_thanh::dem(&d.chu, &mut dem);
        }
    }
    kq.dem_kieu = dem;
    kq.kieu_dau = dem.kieu_chinh();
    bao.buoc(format!(
        "Đọc xong {so_doan_doc} đoạn trong {:.1} giây — kiểu đặt dấu: {} ({} cũ / {} mới)",
        bao.giay(),
        if kq.kieu_dau == Kieu::Cu { "cũ (hòa)" } else { "mới (hoà)" },
        dem.cu,
        dem.moi
    ));
    if dem.ty_le_thieu_so() > 0.2 {
        bao.canh_bao(format!(
            "sách vốn không nhất quán — {:.0}% viết theo kiểu kia",
            dem.ty_le_thieu_so() * 100.0
        ));
    }

    // ---- Lượt 2: sửa ----
    bao.buoc(if mo_hinh.is_some() {
        "Lượt 2/2 — soát và sửa, có mô hình ngôn ngữ"
    } else {
        "Lượt 2/2 — soát và sửa, không có mô hình ngôn ngữ"
    });
    let bo = BoSoat::moi(tuy_chon, kq.kieu_dau);
    let mut da_sua_file: Vec<usize> = Vec::new();

    for (k, &i) in chi_so.iter().enumerate() {
        let truoc_file = kq.da_sua.len();
        bao.tien_do(
            0.15 + 0.83 * k as f32 / chi_so.len() as f32,
            format!("sửa… {k}/{} file", chi_so.len()),
        );
        let Some(noi_dung) = sach::doc_chuoi(&epub.muc[i].noi_dung) else { continue };
        let doan = quet::quet(&noi_dung);
        let mut va: Vec<(std::ops::Range<usize>, String)> = Vec::new();

        for d in &doan {
            kq.so_doan += 1;
            kq.so_chu += d.chu.chars().count();

            let mut r = bo.soat(&d.chu);
            match mo_hinh {
                Some(mh) => {
                    // Gom rồi báo một lượt: `bao` đang được mượn `&mut` nên
                    // không lồng thêm một lượt mượn nữa vào trong lời gọi này.
                    let mut dong = Vec::new();
                    bo.quyet_bang_mo_hinh(&mut r, mh, &mut |doi, chu| dong.push((doi, chu)));
                    for (doi, chu) in dong {
                        bao.chi_tiet(format!("{} {chu}", if doi { "sửa:" } else { "bỏ: " }));
                    }
                }
                None => bo.quyet_khong_mo_hinh(&mut r),
            }
            if r.chu == d.chu && r.cho_xet.is_empty() {
                continue;
            }

            // Đối chiếu bản trước/sau để biết vá byte nào. Xem `doi_chieu` về
            // lý do không dùng thẳng vị trí trong `da_sua`.
            for khac in doi_chieu::so(&d.chu, &r.chu) {
                match d.ve_file(&khac.cu) {
                    Some(trong_file) => va.push((trong_file, khac.moi)),
                    // Chỗ sửa vắt qua ranh giới thẻ — vá được thì phải xoá thẻ,
                    // mà đó là đổi cấu trúc chứ không phải sửa chính tả.
                    None => kq.vuong_the += 1,
                }
            }
            kq.da_sua.extend(r.da_sua);
            kq.chua_sua.extend(r.cho_xet);
        }

        let them = kq.da_sua.len() - truoc_file;
        if them > 0 {
            bao.chi_tiet(format!("{}: {them} chỗ", epub.muc[i].ten));
        }
        if va.is_empty() {
            continue;
        }
        va.sort_by_key(|(r, _)| (r.start, r.end));
        let mut moi = String::with_capacity(noi_dung.len());
        let mut cuoi = 0usize;
        for (r, chu) in &va {
            if r.start < cuoi {
                kq.vuong_the += 1;
                continue;
            }
            moi.push_str(&noi_dung[cuoi..r.start]);
            // Mã hoá lại thực thể: chữ đưa vào là chữ đã giải mã, mà chỗ ta vá
            // vào là nội dung XML. Bỏ bước này thì một dấu `&` trong bản sửa
            // làm hỏng file.
            moi.push_str(&sach::thuc_the::ma_hoa(chu));
            cuoi = r.end;
        }
        moi.push_str(&noi_dung[cuoi..]);
        epub.muc[i].noi_dung = moi.into_bytes();
        da_sua_file.push(i);
    }

    bao.buoc(format!(
        "Soát xong: {} phép sửa ({} lỗi chính tả) trên {} đoạn, {} chỗ để ngỏ",
        kq.da_sua.len(),
        so_loi_dang_ke(&kq),
        kq.so_doan,
        kq.chua_sua.len()
    ));
    if kq.vuong_the > 0 {
        bao.canh_bao(format!("{} chỗ vướng thẻ HTML nên không vá được", kq.vuong_the));
    }

    bao.tien_do(0.98, "ghi file…");
    bao.buoc(format!("Ghi {} ({} file có thay đổi)", dau_ra.display(), da_sua_file.len()));
    epub.ghi(dau_vao, dau_ra, &da_sua_file)
        .with_context(|| format!("không ghi được {}", dau_ra.display()))?;
    bao.tien_do(1.0, "xong");
    bao.buoc(format!("Xong sau {:.1} giây", bao.giay()));
    Ok(kq)
}

/// Gợi ý tên file ra: `abc.epub` → `abc (đã sửa).epub`.
///
/// Không bao giờ trả về chính đường dẫn vào — bản gốc là thứ duy nhất người
/// dùng có để đối chiếu nếu bộ sửa làm sai.
pub fn ten_ra(dau_vao: &Path) -> std::path::PathBuf {
    let than = dau_vao.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let duoi = dau_vao.extension().map(|s| s.to_string_lossy().to_string()).unwrap_or("epub".into());
    dau_vao.with_file_name(format!("{than} (đã sửa).{duoi}"))
}

/// Số lỗi **thật sự sai chính tả**.
///
/// Bỏ ra ngoài mọi loại chỉ dọn hình thức. Gộp chúng vào thì cuốn sách nào cũng
/// "hàng chục nghìn lỗi", mà phần lớn là khoảng trắng thừa và tổ hợp Unicode —
/// con số ấy không nói lên điều gì về chất lượng bản dịch.
///
/// [`Loai::KieuDau`] cũng bị loại, và đây là chỗ dễ nhầm nhất: `hoá` với `hóa`
/// **đều đúng**. Đo trên một bộ truyện dài thì nó chiếm 7.215 trong 7.731 mục,
/// tức là để lẫn vào thì con số này sai gần mười lần.
pub fn so_loi_dang_ke(kq: &KetQuaSach) -> usize {
    kq.da_sua
        .iter()
        .filter(|s| {
            !matches!(
                s.loai,
                Loai::KhoangTrang | Loai::Unicode | Loai::DauCau | Loai::KieuDau
            )
        })
        .count()
}
