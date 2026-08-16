//! Tầng chuẩn hoá: Unicode, ký tự vô hình, khoảng trắng, dấu câu.
//!
//! Tầng này **không đụng tới chữ nghĩa** — không đổi một chữ cái nào — nên sửa
//! được vô điều kiện. Nó cũng là tầng bắt được nhiều lỗi nhất trong ebook thật:
//! sách convert từ web hay từ Word mang theo đủ thứ rác vô hình, và người gõ
//! tiếng Việt quen đặt khoảng trắng trước dấu phẩy.
//!
//! Cạm bẫy xuyên suốt file này là **dấu chấm và dấu phẩy trong con số**. Tiếng
//! Việt dùng dấu phẩy làm dấu thập phân (`1,5 mét`) và dấu chấm phân nhóm hàng
//! nghìn (`1.000 đồng`), đúng ngược với tiếng Anh. Nên mọi luật "thêm khoảng
//! trắng sau dấu" phải bỏ qua khi hai bên đều là chữ số, không thì cả cuốn sách
//! bị chèn khoảng trắng vào giữa mọi con số.

use crate::sua::{DoChac, Loai, SuaDoi};
use unicode_normalization::UnicodeNormalization;

/// Những gì được phép sửa. Mấy mục cuối mặc định tắt vì chúng đổi **kiểu chữ**
/// chứ không sửa lỗi — người biên tập có thể cố ý chọn kiểu ấy.
#[derive(Debug, Clone)]
pub struct CaiDat {
    /// Dựng lại tổ hợp Unicode (NFC) và bỏ ký tự vô hình.
    pub unicode: bool,
    /// Gộp khoảng trắng lặp, bỏ khoảng trắng đầu/cuối đoạn.
    pub khoang_trang: bool,
    /// Khoảng trắng quanh dấu câu.
    pub dau_cau: bool,
    /// Gộp bốn chấm trở lên về ba. Hai chấm để nguyên vì có thể là cố ý.
    pub gom_dau_cham: bool,
    /// Đổi ba chấm `...` thành một ký tự `…`. Mặc định **tắt**: đây là chuyện
    /// trình bày, và nhiều máy đọc sách hiển thị `…` hẹp quá.
    pub dung_ky_tu_ba_cham: bool,
    /// Đổi nháy thẳng `"` `'` thành nháy cong. Mặc định **tắt**: đoán sai chiều
    /// mở/đóng thì hỏng cả đoạn, mà sách dùng nháy thẳng vẫn đọc được.
    pub nhay_cong: bool,
}

impl Default for CaiDat {
    fn default() -> Self {
        CaiDat {
            unicode: true,
            khoang_trang: true,
            dau_cau: true,
            gom_dau_cham: true,
            dung_ky_tu_ba_cham: false,
            nhay_cong: false,
        }
    }
}

/// Ký tự vô hình cần bỏ hẳn.
///
/// `U+00AD` (gạch nối mềm) là thủ phạm âm thầm nhất: nó vô hình trên màn hình
/// nhưng nằm giữa chữ, nên `khô<AD>ng` không khớp với `không` ở bất cứ phép so
/// nào — kể cả phép kiểm chính tả bên dưới. Word chèn nó khi ngắt dòng.
const VO_HINH: [char; 6] = [
    '\u{200B}', // khoảng trắng rộng bằng không
    '\u{200C}', // không nối
    '\u{200D}', // nối
    '\u{FEFF}', // dấu thứ tự byte lạc vào giữa file
    '\u{00AD}', // gạch nối mềm
    '\u{2060}', // nối từ
];

/// Ký tự khoảng trắng lạ cần đưa về khoảng trắng thường.
fn la_khoang_trang_la(c: char) -> bool {
    matches!(
        c,
        '\u{00A0}' | '\u{2002}'..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}' | '\t'
    )
}

/// Dấu câu không được có khoảng trắng đứng trước.
const DAU_DONG_TRUOC: [char; 10] = [',', '.', ';', ':', '!', '?', '…', ')', ']', '}'];

/// Dấu mở không được có khoảng trắng đứng sau.
const DAU_MO: [char; 3] = ['(', '[', '{'];

/// Bước 1 — Unicode. Trả về danh sách phép sửa trên `van_ban`.
pub fn soat_unicode(van_ban: &str, cd: &CaiDat) -> Vec<SuaDoi> {
    if !cd.unicode {
        return Vec::new();
    }
    let mut ra = Vec::new();
    for (vt, c) in van_ban.char_indices() {
        if VO_HINH.contains(&c) {
            ra.push(SuaDoi::moi(
                vt..vt + c.len_utf8(),
                c.to_string(),
                "",
                Loai::Unicode,
                DoChac::Chac,
                format!("bỏ ký tự vô hình U+{:04X}", c as u32),
            ));
        } else if la_khoang_trang_la(c) {
            ra.push(SuaDoi::moi(
                vt..vt + c.len_utf8(),
                c.to_string(),
                " ",
                Loai::Unicode,
                DoChac::Chac,
                format!("U+{:04X} thành khoảng trắng thường", c as u32),
            ));
        }
    }
    ra
}

/// Bước 1b — dựng lại tổ hợp NFC.
///
/// Tách riêng khỏi [`soat_unicode`] vì NFC gộp nhiều ký tự thành một nên đổi
/// hẳn độ dài chuỗi; làm chung với các luật theo vị trí byte thì lệch hết.
/// Trả `None` khi văn bản đã ở dạng NFC — trường hợp thường gặp nhất.
pub fn dung_lai_nfc(van_ban: &str) -> Option<String> {
    let nfc: String = van_ban.nfc().collect();
    if nfc == van_ban {
        None
    } else {
        Some(nfc)
    }
}

/// Bước 2 — khoảng trắng lặp và khoảng trắng thừa ở hai đầu.
pub fn soat_khoang_trang(van_ban: &str, cd: &CaiDat) -> Vec<SuaDoi> {
    if !cd.khoang_trang {
        return Vec::new();
    }
    let mut ra = Vec::new();
    let b = van_ban.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        if b[i] == b' ' {
            let dau = i;
            while i < b.len() && b[i] == b' ' {
                i += 1;
            }
            if i - dau > 1 {
                ra.push(SuaDoi::moi(
                    dau..i,
                    " ".repeat(i - dau),
                    " ",
                    Loai::KhoangTrang,
                    DoChac::Chac,
                    format!("gộp {} khoảng trắng", i - dau),
                ));
            }
        } else {
            i += 1;
        }
    }
    // Hai đầu đoạn. Làm sau nên có thể chồng lên phép gộp ở trên; `ap_dung` giữ
    // phép đứng trước, mà phép đầu/cuối đoạn mới là phép đúng hơn — nên bỏ hẳn
    // phép gộp nào nằm gọn trong phần sắp bị cắt.
    let dau_trong = van_ban.len() - van_ban.trim_start_matches(' ').len();
    let cuoi_trong = van_ban.len() - van_ban.trim_end_matches(' ').len();
    if dau_trong > 0 {
        ra.retain(|s| s.pham_vi.start >= dau_trong);
        ra.push(SuaDoi::moi(
            0..dau_trong,
            " ".repeat(dau_trong),
            "",
            Loai::KhoangTrang,
            DoChac::Chac,
            "bỏ khoảng trắng đầu đoạn",
        ));
    }
    if cuoi_trong > 0 && cuoi_trong < van_ban.len() {
        let bd = van_ban.len() - cuoi_trong;
        ra.retain(|s| s.pham_vi.end <= bd);
        ra.push(SuaDoi::moi(
            bd..van_ban.len(),
            " ".repeat(cuoi_trong),
            "",
            Loai::KhoangTrang,
            DoChac::Chac,
            "bỏ khoảng trắng cuối đoạn",
        ));
    }
    ra
}

/// Bước 3 — dấu câu.
pub fn soat_dau_cau(van_ban: &str, cd: &CaiDat) -> Vec<SuaDoi> {
    if !cd.dau_cau {
        return Vec::new();
    }
    let ky_tu: Vec<(usize, char)> = van_ban.char_indices().collect();
    let mut ra = Vec::new();

    for k in 0..ky_tu.len() {
        let (vt, c) = ky_tu[k];
        let truoc = if k > 0 { Some(ky_tu[k - 1].1) } else { None };
        let sau = ky_tu.get(k + 1).map(|&(_, c)| c);

        // 3a. Khoảng trắng đứng trước dấu đóng câu.
        if c == ' ' && sau.is_some_and(|s| DAU_DONG_TRUOC.contains(&s)) {
            // Trừ dấu chấm mở đầu một cụm ba chấm đứng riêng (` ... `), vốn là
            // cách viết chỗ ngắt lời hợp lệ.
            let ba_cham = sau == Some('.') && ky_tu.get(k + 2).map(|&(_, c)| c) == Some('.');
            if !ba_cham {
                ra.push(SuaDoi::moi(
                    vt..vt + 1,
                    " ",
                    "",
                    Loai::DauCau,
                    DoChac::Chac,
                    format!("bỏ khoảng trắng trước dấu `{}`", sau.unwrap()),
                ));
            }
        }

        // 3b. Khoảng trắng đứng sau dấu mở.
        if c == ' ' && truoc.is_some_and(|t| DAU_MO.contains(&t)) {
            ra.push(SuaDoi::moi(
                vt..vt + 1,
                " ",
                "",
                Loai::DauCau,
                DoChac::Chac,
                format!("bỏ khoảng trắng sau dấu `{}`", truoc.unwrap()),
            ));
        }

        // 3c. Thiếu khoảng trắng sau dấu phẩy / chấm phẩy / hai chấm.
        //
        // Chặn khi cả hai bên đều là chữ số: `1,5` và `10:30` là số và giờ,
        // không phải câu. Đây là luật quan trọng nhất trong file — thiếu nó thì
        // mọi con số trong sách bị tách đôi.
        if matches!(c, ',' | ';' | ':') {
            if let (Some(t), Some(s)) = (truoc, sau) {
                if s != ' ' && !(t.is_ascii_digit() && s.is_ascii_digit()) && !DAU_DONG_TRUOC.contains(&s)
                {
                    ra.push(SuaDoi::moi(
                        vt..vt + c.len_utf8(),
                        c.to_string(),
                        format!("{c} "),
                        Loai::DauCau,
                        DoChac::Chac,
                        format!("thêm khoảng trắng sau dấu `{c}`"),
                    ));
                }
            }
        }

        // 3d. Thiếu khoảng trắng sau dấu kết câu.
        //
        // Chặt hơn 3c nhiều, vì dấu chấm còn dùng để phân nhóm số (`1.000`),
        // viết tắt (`TP.HCM`, `v.v.`) và tên file. Chỉ nhận khi **chữ thường
        // đứng trước, chữ hoa đứng sau** — đó là hình dạng của hai câu dính
        // nhau, và không hình dạng nào ở trên giống thế.
        if matches!(c, '.' | '!' | '?') {
            if let (Some(t), Some(s)) = (truoc, sau) {
                let noi_hai_cau = t.is_lowercase()
                    && s.is_uppercase()
                    && !matches!(ky_tu.get(k + 1).map(|&(_, c)| c), Some('.'))
                    && ky_tu.get(k.wrapping_sub(1)).is_some();
                // `v.v.Nhưng` thì `t` là `v` — vẫn nhận, và nhận đúng. Nhưng
                // `N.A.Trần` thì `t` là `A` viết hoa nên đã tự loại ở trên.
                let cham_lien = k + 1 < ky_tu.len() && ky_tu[k].1 == '.' && truoc == Some('.');
                if noi_hai_cau && !cham_lien {
                    ra.push(SuaDoi::moi(
                        vt..vt + c.len_utf8(),
                        c.to_string(),
                        format!("{c} "),
                        Loai::DauCau,
                        DoChac::KhaChac,
                        format!("thêm khoảng trắng sau dấu `{c}` giữa hai câu"),
                    ));
                }
            }
        }
    }

    if cd.gom_dau_cham {
        ra.extend(gom_dau_cham(van_ban));
    }
    if cd.dung_ky_tu_ba_cham {
        ra.extend(doi_ba_cham(van_ban));
    }
    ra
}

/// Bốn dấu chấm trở lên gộp về ba.
///
/// Hai dấu để nguyên: người ta có khi gõ `..` cố ý, mà đổi thành `...` thì thêm
/// một ký tự vào chỗ tác giả không viết. Bốn trở lên thì chắc là giữ phím.
fn gom_dau_cham(van_ban: &str) -> Vec<SuaDoi> {
    let mut ra = Vec::new();
    let b = van_ban.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        if b[i] == b'.' {
            let dau = i;
            while i < b.len() && b[i] == b'.' {
                i += 1;
            }
            if i - dau >= 4 {
                ra.push(SuaDoi::moi(
                    dau..i,
                    ".".repeat(i - dau),
                    "...",
                    Loai::DauCau,
                    DoChac::Chac,
                    format!("gộp {} dấu chấm về ba", i - dau),
                ));
            }
        } else {
            i += 1;
        }
    }
    ra
}

fn doi_ba_cham(van_ban: &str) -> Vec<SuaDoi> {
    let mut ra = Vec::new();
    let b = van_ban.as_bytes();
    let mut i = 0usize;
    while i + 2 < b.len() {
        if &b[i..i + 3] == b"..." && b.get(i + 3) != Some(&b'.') && (i == 0 || b[i - 1] != b'.') {
            ra.push(SuaDoi::moi(
                i..i + 3,
                "...",
                "…",
                Loai::DauCau,
                DoChac::Chac,
                "dùng ký tự ba chấm",
            ));
            i += 3;
        } else {
            i += 1;
        }
    }
    ra
}

#[cfg(test)]
mod kiem {
    use super::*;
    use crate::sua::ap_dung;

    fn chay(v: &str) -> String {
        let cd = CaiDat::default();
        let mut s = soat_unicode(v, &cd);
        let (v, _) = ap_dung(v, &mut s);
        let v = dung_lai_nfc(&v).unwrap_or(v);
        let mut s = soat_dau_cau(&v, &cd);
        let (v, _) = ap_dung(&v, &mut s);
        let mut s = soat_khoang_trang(&v, &cd);
        let (v, _) = ap_dung(&v, &mut s);
        v
    }

    #[test]
    fn don_rac_vo_hinh() {
        assert_eq!(chay("khô\u{00AD}ng"), "không");
        assert_eq!(chay("a\u{200B}b"), "ab");
        assert_eq!(chay("a\u{00A0}b"), "a b");
    }

    #[test]
    fn dung_lai_to_hop_nfc() {
        // `ế` gõ rời: e + dấu mũ + dấu sắc. Nhìn giống hệt chữ đúng nhưng mọi
        // phép so chuỗi đều trượt.
        let roi = "vi\u{0065}\u{0302}\u{0301}t";
        assert_eq!(chay(roi), "viết");
    }

    #[test]
    fn khoang_trang_quanh_dau_cau() {
        assert_eq!(chay("Anh ấy nói , rồi đi ."), "Anh ấy nói, rồi đi.");
        assert_eq!(chay("một,hai;ba"), "một, hai; ba");
        assert_eq!(chay("( trong ngoặc )"), "(trong ngoặc)");
        assert_eq!(chay("Xong.Rồi đi."), "Xong. Rồi đi.");
    }

    #[test]
    fn khong_dung_vao_con_so() {
        // Luật quan trọng nhất: dấu phẩy thập phân và dấu chấm hàng nghìn kiểu
        // Việt Nam phải sống sót nguyên vẹn.
        assert_eq!(chay("Giá 1,5 triệu và 12.000 đồng lúc 10:30."), "Giá 1,5 triệu và 12.000 đồng lúc 10:30.");
        assert_eq!(chay("Xem TP.HCM và v.v. rồi thôi."), "Xem TP.HCM và v.v. rồi thôi.");
    }

    #[test]
    fn gop_khoang_trang_lap() {
        assert_eq!(chay("một   hai"), "một hai");
        assert_eq!(chay("  đầu và cuối  "), "đầu và cuối");
    }

    #[test]
    fn giu_nguyen_van_ban_da_sach() {
        // Bài kiểm quan trọng nhất của cả tầng: chạy qua văn bản đúng thì không
        // được đổi một byte nào. Ứng dụng tự sửa nên mỗi phép sửa thừa là một
        // chỗ hỏng người dùng không kịp chặn.
        for v in [
            "Anh ấy nói: “Tôi không biết.” Rồi bỏ đi.",
            "Chương 1. Ngày đầu tiên",
            "Giá 1,5 triệu (đã gồm thuế) — rẻ hơn 12.000 đồng.",
            "Thế à? Ừ! Đi thôi...",
        ] {
            assert_eq!(chay(v), v, "đã đổi văn bản vốn đúng");
        }
    }
}
