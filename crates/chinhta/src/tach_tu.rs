//! Cắt văn bản thành từ, kèm vị trí byte.
//!
//! "Từ" ở đây là **một tiếng** — chuỗi chữ cái liền nhau — chứ không phải từ
//! ghép. Tiếng Việt viết rời từng tiếng nên đây là đơn vị tự nhiên để kiểm
//! chính tả; từ ghép thì tầng ngữ cảnh lo.
//!
//! Phần khó không phải là cắt, mà là **biết tiếng nào không đáng kiểm**. Sách
//! dịch đầy tên riêng nước ngoài, sách kỹ thuật đầy thuật ngữ Anh — mà mọi thứ
//! ấy đều trượt phép kiểm cấu tạo âm tiết tiếng Việt. Nếu đem sửa hết thì
//! `Dumbledore` thành một thứ gì đó rất Việt Nam. [`dang_tu`] tách chúng ra.

use crate::am_tiet::{la_chu_viet, la_nguyen_am};

/// Một tiếng đã cắt ra khỏi văn bản.
#[derive(Debug, Clone)]
pub struct Tu<'a> {
    pub chu: &'a str,
    /// Khoảng byte trong văn bản gốc.
    pub dau: usize,
    pub cuoi: usize,
}

/// Loại của một tiếng — quyết định có đem đi kiểm chính tả hay không.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DangTu {
    /// Chữ thuần tiếng Việt: chỉ gồm chữ cái, và có mang dấu tiếng Việt.
    /// Sai chính tả ở đây là sai thật.
    TiengViet,
    /// Chỉ gồm chữ cái ASCII, không dấu. Có thể là tiếng Việt chưa bỏ dấu
    /// (`khong`), có thể là từ tiếng Anh (`window`). Không phân được nếu chỉ
    /// nhìn một tiếng — để tầng ngữ cảnh quyết.
    KhongDau,
    /// Có chữ số, gạch nối, hoặc ký tự ngoài bảng chữ cái tiếng Việt.
    /// Không đụng tới.
    KhongPhaiChu,
    /// Viết hoa toàn bộ. Thường là viết tắt, không kiểm chính tả.
    VietTat,
}

/// Cắt văn bản thành các tiếng.
///
/// Gạch nối **được coi là ranh giới**: `ki-lô-mét` cắt thành ba tiếng, mỗi
/// tiếng kiểm riêng. Gộp lại thì không tiếng nào hợp lệ và cả từ bị báo sai.
pub fn cat(van_ban: &str) -> Vec<Tu<'_>> {
    let mut ra = Vec::new();
    let mut dau: Option<usize> = None;
    for (vt, c) in van_ban.char_indices() {
        if la_chu_viet(c) {
            dau.get_or_insert(vt);
        } else if let Some(d) = dau.take() {
            ra.push(Tu { chu: &van_ban[d..vt], dau: d, cuoi: vt });
        }
    }
    if let Some(d) = dau {
        ra.push(Tu { chu: &van_ban[d..], dau: d, cuoi: van_ban.len() });
    }
    ra
}

/// Tiếng này thuộc dạng nào.
pub fn dang_tu(chu: &str) -> DangTu {
    if chu.is_empty() {
        return DangTu::KhongPhaiChu;
    }
    if !chu.chars().all(la_chu_viet) {
        return DangTu::KhongPhaiChu;
    }
    let chu_cai: Vec<char> = chu.chars().collect();
    if chu_cai.len() > 1 && chu_cai.iter().all(|c| c.is_uppercase()) {
        return DangTu::VietTat;
    }
    // Không có nguyên âm thì không phải tiếng, dù mang dấu hay không. Phép thử
    // này phải đứng **trước** phép thử dấu: chữ `đ` đứng một mình là chữ Việt
    // về mặt ký tự nhưng không phải một tiếng, mà xếp nó vào `TiengViet` thì bộ
    // kiểm âm tiết bắt nó rồi bộ sửa gán cho một nguyên âm — gặp 50 lần trong
    // ba cuốn sách đo được.
    if !chu.chars().any(la_nguyen_am) {
        return DangTu::KhongPhaiChu;
    }
    // Có dấu tiếng Việt — dấu thanh hoặc dấu phụ — thì chắc chắn là chữ Việt.
    // Đây là phép thử duy nhất đáng tin khi chỉ nhìn một tiếng: không ngôn ngữ
    // nào khác trong sách tiếng Việt dùng `ă â ê ô ơ ư đ` hay dấu thanh.
    if chu.chars().any(|c| !c.is_ascii_alphabetic()) {
        DangTu::TiengViet
    } else {
        DangTu::KhongDau
    }
}

#[cfg(test)]
mod kiem {
    use super::*;

    #[test]
    fn cat_dung_vi_tri() {
        let v = "Anh ấy nói: “không”.";
        let tu = cat(v);
        let chu: Vec<&str> = tu.iter().map(|t| t.chu).collect();
        assert_eq!(chu, ["Anh", "ấy", "nói", "không"]);
        for t in &tu {
            assert_eq!(&v[t.dau..t.cuoi], t.chu, "vị trí byte lệch");
        }
    }

    #[test]
    fn gach_noi_la_ranh_gioi() {
        let chu: Vec<&str> = cat("ki-lô-mét").iter().map(|t| t.chu).collect();
        assert_eq!(chu, ["ki", "lô", "mét"]);
    }

    #[test]
    fn phan_biet_dang_tu() {
        assert_eq!(dang_tu("không"), DangTu::TiengViet);
        assert_eq!(dang_tu("đi"), DangTu::TiengViet);
        assert_eq!(dang_tu("window"), DangTu::KhongDau);
        assert_eq!(dang_tu("khong"), DangTu::KhongDau);
        assert_eq!(dang_tu("USB"), DangTu::VietTat);
        assert_eq!(dang_tu("Dumbledore"), DangTu::KhongDau);
        // Chữ cái đứng một mình không phải một tiếng, dù là chữ Việt.
        assert_eq!(dang_tu("đ"), DangTu::KhongPhaiChu);
        assert_eq!(dang_tu("ngh"), DangTu::KhongPhaiChu);
    }
}
