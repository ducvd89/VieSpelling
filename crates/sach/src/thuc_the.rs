//! Thực thể XML (`&amp;`, `&#x2014;`) — giải mã để soi chữ, mã hoá lại để ghi.
//!
//! Vì sao phải làm hẳn một vòng giải-rồi-mã: phép kiểm chính tả cần thấy **chữ**
//! chứ không thấy `&#273;`. Mà `&nbsp;` giải ra U+00A0 lại đúng là thứ tầng
//! chuẩn hoá phải dọn. Không giải mã thì hai lớp lỗi ấy vô hình.
//!
//! Chiều ngược lại thì mã hoá **ít nhất có thể**: chỉ `&`, `<`, `>`. Chữ tiếng
//! Việt ghi thẳng UTF-8, vì EPUB bắt buộc UTF-8 nên không có gì để lo, mà đổi
//! `đ` thành `&#273;` chỉ làm file phình ra và khác hẳn bản gốc.

/// Giải mã thực thể XML trong một đoạn chữ.
///
/// Chỉ nhận năm thực thể có tên mà XML định nghĩa sẵn, cộng `&nbsp;` (HTML định
/// nghĩa, nhưng EPUB2 dùng đầy) và thực thể số. Tên lạ để nguyên: EPUB nào khai
/// thực thể riêng trong DTD thì ta không đọc DTD, đổi bừa là hỏng.
pub fn giai_ma(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut ra = String::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        if b[i] != b'&' {
            let dau = i;
            while i < b.len() && b[i] != b'&' {
                i += 1;
            }
            ra.push_str(&s[dau..i]);
            continue;
        }
        // Thực thể dài nhất ta nhận là `&#x10FFFF;` — 10 ký tự. Quét quá đó thì
        // đây là dấu `&` đứng một mình, khá thường gặp trong sách.
        let het = s[i..].find(';').map(|k| i + k);
        match het {
            Some(h) if h - i <= 10 => {
                let ten = &s[i + 1..h];
                match doi_ten(ten) {
                    Some(c) => {
                        ra.push(c);
                        i = h + 1;
                    }
                    None => {
                        ra.push('&');
                        i += 1;
                    }
                }
            }
            _ => {
                ra.push('&');
                i += 1;
            }
        }
    }
    ra
}

/// Giải mã kèm **bản đồ vị trí ngược**: `ban_do[i]` là vị trí byte trong chuỗi
/// gốc của byte thứ `i` trong chuỗi đã giải.
///
/// Cần bản đồ vì phép sửa tìm ra trên chữ đã giải mã, mà lúc ghi lại phải vá
/// đúng khoảng byte trong file gốc. Không có bản đồ thì một đoạn chứa `&amp;`
/// làm lệch mọi vị trí phía sau nó.
///
/// Trả `None` cho bản đồ khi chuỗi không có thực thể nào — lúc ấy vị trí trùng
/// khít, khỏi tốn bộ nhớ. Đây là trường hợp của gần như mọi đoạn văn.
pub fn giai_ma_co_ban_do(s: &str) -> (String, Option<Vec<u32>>) {
    if !s.contains('&') {
        return (s.to_string(), None);
    }
    let mut ra = String::with_capacity(s.len());
    let mut ban_do: Vec<u32> = Vec::with_capacity(s.len() + 1);
    let b = s.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        if b[i] != b'&' {
            let dau = i;
            while i < b.len() && b[i] != b'&' {
                i += 1;
            }
            for k in dau..i {
                ban_do.push(k as u32);
            }
            ra.push_str(&s[dau..i]);
            continue;
        }
        let het = s[i..].find(';').map(|k| i + k);
        match het.filter(|&h| h - i <= 10).and_then(|h| doi_ten(&s[i + 1..h]).map(|c| (c, h))) {
            Some((c, h)) => {
                // Cả thực thể thu về một ký tự: mọi byte của ký tự ấy đều trỏ
                // về vị trí dấu `&`. Phép sửa nào rơi vào giữa sẽ được cắt ra
                // đúng biên thực thể, không xẻ đôi nó.
                let truoc = ra.len();
                ra.push(c);
                for _ in truoc..ra.len() {
                    ban_do.push(i as u32);
                }
                i = h + 1;
            }
            None => {
                ban_do.push(i as u32);
                ra.push('&');
                i += 1;
            }
        }
    }
    ban_do.push(s.len() as u32);
    (ra, Some(ban_do))
}

fn doi_ten(ten: &str) -> Option<char> {
    match ten {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{00A0}'),
        _ => {
            let so = ten.strip_prefix('#')?;
            let ma = match so.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => so.parse::<u32>().ok()?,
            };
            char::from_u32(ma)
        }
    }
}

/// Mã hoá lại để nhét vào nội dung XML.
pub fn ma_hoa(s: &str) -> String {
    let mut ra = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => ra.push_str("&amp;"),
            '<' => ra.push_str("&lt;"),
            '>' => ra.push_str("&gt;"),
            k => ra.push(k),
        }
    }
    ra
}

#[cfg(test)]
mod kiem {
    use super::*;

    #[test]
    fn giai_ma_cac_dang() {
        assert_eq!(giai_ma("a&amp;b"), "a&b");
        assert_eq!(giai_ma("&#273;i"), "đi");
        assert_eq!(giai_ma("&#x111;i"), "đi");
        assert_eq!(giai_ma("a&nbsp;b"), "a\u{00A0}b");
    }

    #[test]
    fn dau_va_don_le_song_sot() {
        // `&` đứng một mình là XML không hợp lệ nhưng EPUB ngoài đời đầy. Không
        // được nuốt mất nó, cũng không được đoán bừa.
        assert_eq!(giai_ma("Tom & Jerry"), "Tom & Jerry");
        assert_eq!(giai_ma("&khonghieu;"), "&khonghieu;");
    }

    #[test]
    fn ma_hoa_roi_giai_ma_ra_chinh_no() {
        for v in ["Tom & Jerry", "a<b>c", "chữ Việt đầy đủ dấu", "&amp; sẵn"] {
            assert_eq!(giai_ma(&ma_hoa(v)), v);
        }
    }
}
