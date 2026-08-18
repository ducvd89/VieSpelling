//! Cài đặt của người dùng, lưu xuống đĩa.

use chinhta::chuan_hoa::CaiDat as ChuanHoa;
use chinhta::soat::TuyChon;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CaiDat {
    pub mo_hinh: Option<PathBuf>,
    pub unicode: bool,
    pub khoang_trang: bool,
    pub dau_cau: bool,
    pub gom_dau_cham: bool,
    pub dung_ky_tu_ba_cham: bool,
    pub nhat_quan_dau_thanh: bool,
    pub am_tiet_sai: bool,
    pub de_nham: bool,
    pub chu_khong_dau: bool,
    pub nguong_mo_hinh: f32,
    /// Viết file báo cáo HTML cạnh sách đã sửa.
    pub viet_bao_cao: bool,
}

impl Default for CaiDat {
    fn default() -> Self {
        let m = TuyChon::default();
        CaiDat {
            mo_hinh: None,
            unicode: m.chuan_hoa.unicode,
            khoang_trang: m.chuan_hoa.khoang_trang,
            dau_cau: m.chuan_hoa.dau_cau,
            gom_dau_cham: m.chuan_hoa.gom_dau_cham,
            dung_ky_tu_ba_cham: m.chuan_hoa.dung_ky_tu_ba_cham,
            nhat_quan_dau_thanh: m.nhat_quan_dau_thanh,
            am_tiet_sai: m.am_tiet_sai,
            de_nham: m.de_nham,
            chu_khong_dau: m.chu_khong_dau,
            nguong_mo_hinh: m.nguong_mo_hinh,
            viet_bao_cao: true,
        }
    }
}

impl CaiDat {
    pub fn thanh_tuy_chon(&self) -> TuyChon {
        TuyChon {
            chuan_hoa: ChuanHoa {
                unicode: self.unicode,
                khoang_trang: self.khoang_trang,
                dau_cau: self.dau_cau,
                gom_dau_cham: self.gom_dau_cham,
                dung_ky_tu_ba_cham: self.dung_ky_tu_ba_cham,
                nhay_cong: false,
            },
            nhat_quan_dau_thanh: self.nhat_quan_dau_thanh,
            am_tiet_sai: self.am_tiet_sai,
            de_nham: self.de_nham,
            chu_khong_dau: self.chu_khong_dau,
            nguong_mo_hinh: self.nguong_mo_hinh,
            // Bản cửa sổ giữ đúng một lối chấm — lối đã đo là tốt hơn. Một cái
            // hộp đánh dấu "đổi cách hỏi mô hình" thì người dùng không có cách
            // nào chọn đúng: chọn xong cũng chẳng thấy gì khác ngoài thời gian
            // chạy, còn cái khác thật thì nằm ở loại lỗi sai, phải đo cả cuốn
            // sách mới thấy. Muốn so lại thì dùng `vsc --ca-cau`.
            kieu_cham: chinhta::soat::KieuCham::default(),
        }
    }

    fn duong_dan() -> Option<PathBuf> {
        let goc = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(goc.join("VieSpellcheck").join("cai-dat.json"))
    }

    pub fn nap() -> CaiDat {
        // Cài đặt hỏng thì dùng mặc định chứ không dừng ứng dụng — mất cài đặt
        // là phiền, không mở được ứng dụng mới là hỏng.
        Self::duong_dan()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn luu(&self) {
        let Some(p) = Self::duong_dan() else { return };
        if let Some(cha) = p.parent() {
            let _ = std::fs::create_dir_all(cha);
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(p, s);
        }
    }
}
