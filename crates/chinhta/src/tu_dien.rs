//! Từ điển tiếng Việt: kho âm tiết có thật và kho từ ghép.
//!
//! # Vì sao từ điển làm phép kiểm chính, còn bảng vần lùi về dự phòng
//!
//! [`crate::am_tiet`] kiểm bằng **cấu tạo**: âm đầu + vần + thanh. Cách ấy gọn
//! và giải thích được, nhưng nó có một điểm mù không vá nổi — tiếng Việt hiện
//! đại đầy từ mượn viết theo âm Việt, mà chúng mang hình dạng ngoài hệ thống
//! ngữ âm: `bêtông`, `cafê`, `micrô`, `pittông`, `rađa`, `nilông`, `blô`,
//! `phrăng`. Đo trên chính từ điển này thì **544 âm tiết có dấu tiếng Việt** bị
//! bảng vần bác bỏ, và phần lớn là từ mượn như thế.
//!
//! Mỗi mục bị bác oan không phải là bỏ sót — ứng dụng tự sửa, nên nó là một chữ
//! **đúng** bị đổi thành chữ **sai**. Đắt hơn hẳn việc bỏ qua một lỗi.
//!
//! Nên thứ tự là: có trong từ điển thì thôi, không có thì mới hỏi bảng vần.
//! Bảng vần vẫn cần, vì từ điển không phủ hết tên riêng và từ mới, và vì phần
//! sinh ứng viên sửa cần biết **chẻ** một tiếng ra thế nào chứ không chỉ biết
//! nó có tồn tại hay không.
//!
//! # Kho từ ghép dùng để làm gì
//!
//! Đây là thứ chữa lớp lỗi mà mô hình ngôn ngữ chọn sai. `chúg ta` sinh ra hàng
//! chục ứng viên đều là tiếng có thật — `chúng`, `chừ`, `chú`, `chug`… — và mô
//! hình 9 tỷ tham số vẫn chọn `chừ`. Nhưng `chúng ta` có trong từ điển còn
//! `chừ ta` thì không, và bằng chứng ấy dứt khoát hơn hẳn mọi điểm số.
//!
//! Hai file dữ liệu dựng bằng `examples/dung_tu_dien.rs` từ ba bộ từ điển
//! (tudientv, Wiktionary tiếng Việt, Hồ Ngọc Đức) — xem `du-lieu/NGUON.md`.

use std::collections::HashSet;
use std::sync::OnceLock;

const AM_TIET: &str = include_str!("../../../du-lieu/am-tiet.txt");
const TU_GHEP: &str = include_str!("../../../du-lieu/tu-ghep.txt");

fn kho_am_tiet() -> &'static HashSet<&'static str> {
    static KHO: OnceLock<HashSet<&'static str>> = OnceLock::new();
    KHO.get_or_init(|| AM_TIET.lines().filter(|l| !l.is_empty()).collect())
}

fn kho_tu_ghep() -> &'static HashSet<&'static str> {
    static KHO: OnceLock<HashSet<&'static str>> = OnceLock::new();
    KHO.get_or_init(|| TU_GHEP.lines().filter(|l| !l.is_empty()).collect())
}

/// Tiếng này có trong từ điển không. Không phân biệt hoa thường.
///
/// Nhận `&str` đã viết thường thì nhanh hơn; hàm tự hạ chữ nếu cần.
pub fn co_am_tiet(tieng: &str) -> bool {
    if tieng.is_empty() {
        return false;
    }
    if kho_am_tiet().contains(tieng) {
        return true;
    }
    let thap = tieng.to_lowercase();
    kho_am_tiet().contains(thap.as_str())
}

/// Cụm tiếng này có phải một từ ghép trong từ điển không.
///
/// `cum` là các tiếng đã viết thường, nối bằng đúng một khoảng trắng.
pub fn co_tu_ghep(cum: &str) -> bool {
    kho_tu_ghep().contains(cum)
}

/// Ghép `tieng` với hàng xóm hai bên, xem có ra từ ghép nào trong từ điển không.
///
/// Đây là phép chấm điểm ứng viên rẻ nhất và chắc nhất mà ứng dụng có. Trả về
/// số từ ghép dựng được (0, 1 hoặc 2) — nhiều hơn nghĩa là ứng viên khớp cả hai
/// phía, gần như chắc chắn đúng.
pub fn khop_hang_xom(truoc: Option<&str>, tieng: &str, sau: Option<&str>) -> usize {
    let t = tieng.to_lowercase();
    let mut n = 0;
    if let Some(p) = truoc {
        if co_tu_ghep(&format!("{} {t}", p.to_lowercase())) {
            n += 1;
        }
    }
    if let Some(s) = sau {
        if co_tu_ghep(&format!("{t} {}", s.to_lowercase())) {
            n += 1;
        }
    }
    n
}

pub fn so_am_tiet() -> usize {
    kho_am_tiet().len()
}

pub fn so_tu_ghep() -> usize {
    kho_tu_ghep().len()
}

#[cfg(test)]
mod kiem {
    use super::*;

    #[test]
    fn kho_nap_du() {
        assert!(so_am_tiet() > 9_000, "kho âm tiết quá nhỏ: {}", so_am_tiet());
        assert!(so_tu_ghep() > 60_000, "kho từ ghép quá nhỏ: {}", so_tu_ghep());
    }

    #[test]
    fn nhan_tu_muon_ma_bang_van_bac_bo() {
        // Đây là lý do tầng này tồn tại: những chữ dưới đây là tiếng Việt thật,
        // gặp thường xuyên trong sách, mà không ghép được từ âm đầu + vần nào.
        for t in ["bêtông", "micrô", "pittông", "rađa", "nilông", "cafê"] {
            assert!(co_am_tiet(t), "từ điển thiếu `{t}`");
            assert!(!crate::am_tiet::hop_le(t), "`{t}` mà bảng vần lại nhận?");
        }
    }

    #[test]
    fn nhan_tieng_thuong_gap() {
        for t in ["không", "người", "được", "chúng", "quýt", "méc"] {
            assert!(co_am_tiet(t), "từ điển thiếu `{t}`");
        }
    }

    #[test]
    fn khong_nhan_chuoi_bay() {
        for t in ["thuơng", "khôngg", "xxyyzz", "chúg"] {
            assert!(!co_am_tiet(t), "từ điển nhận nhầm `{t}`");
        }
    }

    #[test]
    fn tu_ghep_phan_biet_duoc_ung_vien() {
        // Ca cụ thể mà mô hình ngôn ngữ chọn sai: `chúg ta`. Từ ghép phân được
        // ngay, không cần card đồ hoạ.
        assert!(co_tu_ghep("chúng ta"));
        assert!(!co_tu_ghep("chừ ta"));
        assert_eq!(khop_hang_xom(None, "chúng", Some("ta")), 1);
        assert_eq!(khop_hang_xom(None, "chừ", Some("ta")), 0);
    }

    #[test]
    fn khop_ca_hai_phia() {
        // `sử` trong `lịch sử học`: khớp cả trái lẫn phải.
        let n = khop_hang_xom(Some("lịch"), "sử", Some("học"));
        assert!(n >= 1, "không khớp phía nào");
    }
}
