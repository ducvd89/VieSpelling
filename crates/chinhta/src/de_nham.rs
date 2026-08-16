//! Cặp dễ nhầm: hai cách viết đều là tiếng Việt hợp lệ, chỉ một cách đúng.
//!
//! Bộ kiểm cấu tạo âm tiết mù hoàn toàn với lớp lỗi này — `xử dụng` gồm hai
//! tiếng viết đúng chuẩn, `chia xẻ` cũng vậy. Chỉ có từ điển cụm từ mới bắt
//! được, nên đây là chỗ duy nhất trong ứng dụng dùng danh sách gõ tay.
//!
//! Bảng nằm ở `du-lieu/de-nham.txt` và nhúng thẳng vào chương trình. Nó chia
//! hai phần theo **việc có cần hiểu câu hay không**, và ranh giới ấy là thứ
//! quyết định phép sửa nào được tự áp:
//!
//! - `[LUON_SAI]` — dạng sai không phải là từ. Tự sửa.
//! - `[TUY_NGHIA]` — cả hai đều là từ. Chỉ đề xuất; mô hình ngôn ngữ quyết.

use crate::sua::{DoChac, Loai, SuaDoi};
use crate::tach_tu;

const BANG: &str = include_str!("../../../du-lieu/de-nham.txt");

pub struct Bang {
    luon_sai: Vec<(Vec<String>, String)>,
    tuy_nghia: Vec<(Vec<String>, String)>,
}

impl Bang {
    pub fn nap() -> Bang {
        let mut luon_sai = Vec::new();
        let mut tuy_nghia = Vec::new();
        let mut trong_tuy_nghia = false;
        for dong in BANG.lines() {
            let d = dong.trim();
            if d.is_empty() || d.starts_with('#') {
                continue;
            }
            if d == "[LUON_SAI]" {
                trong_tuy_nghia = false;
                continue;
            }
            if d == "[TUY_NGHIA]" {
                trong_tuy_nghia = true;
                continue;
            }
            let tach = if trong_tuy_nghia { '|' } else { '\u{0}' };
            let (trai, phai) = if trong_tuy_nghia {
                match d.split_once(tach) {
                    Some(x) => x,
                    None => continue,
                }
            } else {
                match d.split_once("=>") {
                    Some(x) => x,
                    None => continue,
                }
            };
            let khoa: Vec<String> =
                trai.trim().split_whitespace().map(|s| s.to_lowercase()).collect();
            let gia_tri = phai.trim().to_string();
            if khoa.is_empty() || gia_tri.is_empty() || khoa.join(" ") == gia_tri.to_lowercase() {
                continue;
            }
            if trong_tuy_nghia {
                tuy_nghia.push((khoa, gia_tri));
            } else {
                luon_sai.push((khoa, gia_tri));
            }
        }
        // Cụm dài khớp trước: `thái độ bàng quang` phải thắng `bàng quang`.
        luon_sai.sort_by_key(|(k, _)| std::cmp::Reverse(k.len()));
        tuy_nghia.sort_by_key(|(k, _)| std::cmp::Reverse(k.len()));
        Bang { luon_sai, tuy_nghia }
    }

    pub fn so_muc(&self) -> (usize, usize) {
        (self.luon_sai.len(), self.tuy_nghia.len())
    }

    /// Dò một đoạn.
    ///
    /// Trả cả hai loại trong cùng một danh sách; phân biệt bằng [`DoChac`] —
    /// `Chac` là tự sửa, `NgoVuc` là phải hỏi mô hình.
    pub fn soat(&self, van_ban: &str) -> Vec<SuaDoi> {
        let tu = tach_tu::cat(van_ban);
        if tu.is_empty() {
            return Vec::new();
        }
        let thap: Vec<String> = tu.iter().map(|t| t.chu.to_lowercase()).collect();
        let mut ra: Vec<SuaDoi> = Vec::new();
        let mut da_dung = vec![false; tu.len()];

        for (bang, chac) in [
            (&self.luon_sai, DoChac::Chac),
            (&self.tuy_nghia, DoChac::NgoVuc),
        ] {
            for (khoa, dung) in bang.iter() {
                let n = khoa.len();
                if n > tu.len() {
                    continue;
                }
                for i in 0..=tu.len() - n {
                    if da_dung[i..i + n].iter().any(|&x| x) {
                        continue;
                    }
                    if thap[i..i + n] != khoa[..] {
                        continue;
                    }
                    // Cụm phải đứng liền nhau — chỉ cách nhau bằng khoảng
                    // trắng. Có dấu câu xen vào thì không phải cùng một cụm.
                    let lien = (i..i + n - 1).all(|k| {
                        van_ban[tu[k].cuoi..tu[k + 1].dau].chars().all(|c| c == ' ')
                    });
                    if !lien {
                        continue;
                    }
                    let dau = tu[i].dau;
                    let cuoi = tu[i + n - 1].cuoi;
                    let goc = &van_ban[dau..cuoi];
                    let moi = theo_kieu_hoa(goc, dung);
                    if moi == goc {
                        continue;
                    }
                    for x in da_dung[i..i + n].iter_mut() {
                        *x = true;
                    }
                    let ly_do = if chac == DoChac::Chac {
                        format!("`{}` không phải là từ", khoa.join(" "))
                    } else {
                        format!("`{}` và `{}` khác nghĩa — chọn theo câu", khoa.join(" "), dung)
                    };
                    ra.push(SuaDoi::moi(dau..cuoi, goc, moi, Loai::DeNham, chac, ly_do));
                }
            }
        }
        ra
    }
}

/// Chép kiểu viết hoa của bản gốc sang bản sửa.
///
/// Chỉ nhìn chữ cái đầu: cụm dễ nhầm nào cũng nằm giữa câu hoặc đầu câu, chưa
/// gặp ca viết hoa giữa cụm.
fn theo_kieu_hoa(goc: &str, moi: &str) -> String {
    if !goc.chars().next().is_some_and(|c| c.is_uppercase()) {
        return moi.to_string();
    }
    let mut c = moi.chars();
    match c.next() {
        Some(d) => d.to_uppercase().collect::<String>() + c.as_str(),
        None => moi.to_string(),
    }
}

#[cfg(test)]
mod kiem {
    use super::*;
    use crate::sua::ap_dung;

    fn tu_sua(v: &str) -> String {
        let b = Bang::nap();
        // Chỉ áp phần chắc chắn, đúng như ứng dụng làm khi không có mô hình.
        let mut s: Vec<SuaDoi> = b.soat(v).into_iter().filter(|s| s.do_chac == DoChac::Chac).collect();
        ap_dung(v, &mut s).0
    }

    #[test]
    fn bang_nap_duoc() {
        let (a, b) = Bang::nap().so_muc();
        assert!(a > 30, "bảng luôn-sai quá ngắn: {a}");
        assert!(b > 5, "bảng tuỳ-nghĩa quá ngắn: {b}");
    }

    #[test]
    fn sua_dang_luon_sai() {
        assert_eq!(tu_sua("Cách xử dụng máy"), "Cách sử dụng máy");
        assert_eq!(tu_sua("một câu truyện hay"), "một câu chuyện hay");
        assert_eq!(tu_sua("anh ta bắt trước tôi"), "anh ta bắt chước tôi");
    }

    #[test]
    fn khong_tu_sua_dang_tuy_nghia() {
        // `dành cho` là từ thật. Không được tự đổi thành `giành cho`.
        assert_eq!(tu_sua("phần dành cho em"), "phần dành cho em");
        assert_eq!(tu_sua("chia xẻ nỗi buồn"), "chia xẻ nỗi buồn");
        // Nhưng phải *đề xuất* để tầng mô hình xét.
        let b = Bang::nap();
        assert!(b.soat("chia xẻ nỗi buồn").iter().any(|s| s.do_chac == DoChac::NgoVuc));
    }

    #[test]
    fn cum_dai_thang_cum_ngan() {
        assert_eq!(tu_sua("thái độ bàng quang của anh"), "thái độ bàng quan của anh");
        // `bàng quang` đứng một mình là bộ phận cơ thể — không được đụng.
        assert_eq!(tu_sua("viêm bàng quang"), "viêm bàng quang");
    }

    #[test]
    fn giu_kieu_viet_hoa() {
        assert_eq!(tu_sua("Xử dụng thế nào?"), "Sử dụng thế nào?");
    }

    #[test]
    fn dau_cau_xen_giua_thi_khong_phai_cum() {
        assert_eq!(tu_sua("anh xử, dụng cụ kia"), "anh xử, dụng cụ kia");
    }
}
