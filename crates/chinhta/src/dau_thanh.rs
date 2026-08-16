//! Dấu thanh đặt sai chỗ, và chuyện kiểu cũ / kiểu mới.
//!
//! Hai việc khác hẳn nhau tuy cùng biểu hiện là "dấu nằm sai chữ":
//!
//! **Sai thật.** `qúy`, `gía`, `thùơng`. Chữ `u` trong `qu` và chữ `i` trong
//! `gi` thuộc về âm đầu, không phải nguyên âm, nên dấu thanh không rơi vào đó
//! được. Sai này không cần nhìn ngữ cảnh, sửa thẳng.
//!
//! **Khác kiểu.** `hòa` với `hoà`, `thùy` với `thuỳ`. Cả hai đều đúng — kiểu cũ
//! đặt dấu ở nguyên âm đầu, kiểu mới đặt ở nguyên âm chính. Bộ Giáo dục công
//! nhận cả hai. Nên ở đây **không có bên nào sai**, chỉ có chuyện một cuốn sách
//! nên nhất quán. Vì vậy phải **đếm cả sách trước** rồi mới kéo về phe đông
//! hơn; kéo về một kiểu cố định là tự ý áp lựa chọn của mình lên sách người ta.
//!
//! Chỉ ba vần mở `oa`, `oe`, `uy` mới có chuyện hai kiểu. Có âm cuối là hết
//! nhập nhằng: `hoàn` chỉ có một cách viết.

use crate::am_tiet::{self, AmTiet};
use crate::sua::{DoChac, Loai, SuaDoi};
use crate::tach_tu::{self, DangTu};

/// Kiểu đặt dấu của cả cuốn sách, đếm được từ chính nó.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Kieu {
    /// Kiểu tuyệt đại đa số sách in dùng, nên là mặc định khi chưa đếm được gì.
    #[default]
    Cu,
    Moi,
}

/// Đếm phiếu cho hai kiểu đặt dấu.
#[derive(Debug, Default, Clone, Copy)]
pub struct DemKieu {
    pub cu: usize,
    pub moi: usize,
}

impl DemKieu {
    /// Kiểu chiếm đa số. Hoà hoặc không có mẫu nào thì chọn **kiểu cũ**, vì đó
    /// là kiểu tuyệt đại đa số sách in đang dùng.
    pub fn kieu_chinh(&self) -> Kieu {
        if self.moi > self.cu {
            Kieu::Moi
        } else {
            Kieu::Cu
        }
    }

    /// Tỷ lệ của phe thiểu số. Gần 0 nghĩa là sách vốn đã nhất quán, và những
    /// chỗ lẻ tẻ còn lại đúng là lỗi đánh máy.
    pub fn ty_le_thieu_so(&self) -> f32 {
        let tong = self.cu + self.moi;
        if tong == 0 {
            0.0
        } else {
            self.cu.min(self.moi) as f32 / tong as f32
        }
    }
}

/// Vần này có nhập nhằng hai kiểu đặt dấu không.
fn co_hai_kieu(at: &AmTiet) -> bool {
    matches!(at.van.as_str(), "oa" | "oe" | "uy") && at.thanh != am_tiet::NGANG
}

/// Đếm phiếu kiểu đặt dấu trong một đoạn văn bản. Gọi cho mọi đoạn của sách rồi
/// cộng dồn, trước khi chạy [`soat`].
pub fn dem(van_ban: &str, dem: &mut DemKieu) {
    for t in tach_tu::cat(van_ban) {
        if tach_tu::dang_tu(t.chu) != DangTu::TiengViet {
            continue;
        }
        let Some(at) = am_tiet::tach(t.chu) else { continue };
        if !co_hai_kieu(&at) {
            continue;
        }
        let thap = t.chu.to_lowercase();
        if thap == am_tiet::ghep(&at, false).to_lowercase() {
            dem.cu += 1;
        } else if thap == am_tiet::ghep(&at, true).to_lowercase() {
            dem.moi += 1;
        }
    }
}

/// Dò lỗi dấu thanh trong một đoạn.
///
/// `kieu` là kiểu đã đếm được của cả sách. `nhat_quan` bật thì kéo cả phe thiểu
/// số về kiểu chính; tắt thì chỉ sửa lỗi thật và để `hòa`/`hoà` yên.
pub fn soat(van_ban: &str, kieu: Kieu, nhat_quan: bool) -> Vec<SuaDoi> {
    let kieu_moi = kieu == Kieu::Moi;
    let mut ra = Vec::new();
    for t in tach_tu::cat(van_ban) {
        if tach_tu::dang_tu(t.chu) != DangTu::TiengViet {
            continue;
        }
        let Some(at) = am_tiet::tach(t.chu) else { continue };

        let theo_kieu = am_tiet::ghep(&at, kieu_moi);
        if theo_kieu == t.chu {
            continue;
        }
        // **Chỉ so phần chữ, bỏ qua cách viết hoa.** `ghep` chỉ dựng lại được
        // hai kiểu hoa quen thuộc — hoa chữ đầu và hoa hết — nên mọi kiểu khác
        // bị nó nắn về một trong hai. Sách convert từ bản quét đầy chữ kiểu
        // `THầy`, `KHông` (dấu tích của chữ cái hoa to đầu đoạn), và tầng này
        // từng âm thầm đổi 66 chỗ như thế trong một cuốn Harry Potter — dán
        // nhãn "sửa dấu thanh" cho một việc chẳng liên quan gì tới dấu thanh.
        // Đổi cách viết hoa không nằm trong việc được giao.
        if theo_kieu.to_lowercase() == t.chu.to_lowercase() {
            continue;
        }
        // Viết đúng nhưng theo kiểu kia. Đây không phải lỗi.
        let theo_kieu_kia = am_tiet::ghep(&at, !kieu_moi);
        if theo_kieu_kia == t.chu {
            if !nhat_quan || !co_hai_kieu(&at) {
                continue;
            }
            ra.push(SuaDoi::moi(
                t.dau..t.cuoi,
                t.chu,
                theo_kieu,
                Loai::KieuDau,
                DoChac::KhaChac,
                format!(
                    "cả hai đều đúng — kéo về kiểu {} của cả sách",
                    if kieu_moi { "mới" } else { "cũ" }
                ),
            ));
            continue;
        }

        // Không khớp kiểu nào — dấu thanh đặt sai chỗ thật.
        ra.push(SuaDoi::moi(
            t.dau..t.cuoi,
            t.chu,
            theo_kieu,
            Loai::DauThanh,
            DoChac::Chac,
            "dấu thanh đặt sai nguyên âm".to_string(),
        ));
    }
    ra
}

#[cfg(test)]
mod kiem {
    use super::*;
    use crate::sua::ap_dung;

    fn chay(v: &str, kieu: Kieu, nhat_quan: bool) -> String {
        let mut s = soat(v, kieu, nhat_quan);
        ap_dung(v, &mut s).0
    }

    #[test]
    fn sua_dau_dat_sai_sau_qu_va_gi() {
        // Lỗi gõ kinh điển: người ta nhìn `qu` như nguyên âm đôi rồi bỏ dấu vào
        // chữ u. Sai chắc chắn, không phụ thuộc kiểu.
        assert_eq!(chay("qúy", Kieu::Cu, false), "quý");
        assert_eq!(chay("gía", Kieu::Cu, false), "giá");
        assert_eq!(chay("qúa", Kieu::Moi, false), "quá");
    }

    #[test]
    fn khong_dung_vao_van_da_dung() {
        for v in ["quý", "giá", "hòa", "hoàn", "người", "được", "khuỷu"] {
            assert_eq!(chay(v, Kieu::Cu, false), v, "đổi chữ vốn đúng: {v}");
        }
    }

    #[test]
    fn hai_kieu_deu_de_yen_khi_khong_bat_nhat_quan() {
        assert_eq!(chay("hòa thuận", Kieu::Moi, false), "hòa thuận");
        assert_eq!(chay("hoà thuận", Kieu::Cu, false), "hoà thuận");
    }

    #[test]
    fn keo_ve_kieu_la_loai_rieng_khong_phai_loi() {
        // `hoà` và `hòa` đều đúng. Phép kéo về kiểu chung phải mang nhãn riêng,
        // không thì báo cáo đếm nó vào số lỗi chính tả và con số phồng lên gấp
        // mười trên sách dài.
        let s = soat("hoà thuận", Kieu::Cu, true);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].loai, Loai::KieuDau);
        // Còn dấu đặt sai nguyên âm thì vẫn là lỗi thật.
        let s = soat("qúy", Kieu::Cu, true);
        assert_eq!(s[0].loai, Loai::DauThanh);
    }

    #[test]
    fn keo_ve_kieu_chinh_khi_bat_nhat_quan() {
        assert_eq!(chay("hoà thuận", Kieu::Cu, true), "hòa thuận");
        assert_eq!(chay("hòa thuận", Kieu::Moi, true), "hoà thuận");
        // Vần có âm cuối không nhập nhằng nên đừng đụng vào.
        assert_eq!(chay("hoàn toàn", Kieu::Moi, true), "hoàn toàn");
    }

    #[test]
    fn khong_dung_vao_cach_viet_hoa() {
        // Sách quét lại hay có `THầy`, `KHông` — dấu tích của chữ cái to đầu
        // đoạn. Chúng không sai dấu thanh, nên tầng này phải làm ngơ. Từng đổi
        // 66 chỗ như vậy trong một cuốn Harry Potter.
        for v in ["THầy", "KHông", "CHúng", "NHưng"] {
            assert_eq!(chay(v, Kieu::Cu, true), v, "đã đổi cách viết hoa: {v}");
        }
    }

    #[test]
    fn dem_phieu_theo_ca_sach() {
        let mut d = DemKieu::default();
        dem("hòa khòe thùy hoà", &mut d);
        assert_eq!(d.cu, 3);
        assert_eq!(d.moi, 1);
        assert_eq!(d.kieu_chinh(), Kieu::Cu);
    }
}
