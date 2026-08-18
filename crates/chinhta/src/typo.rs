//! Bảng typo hay gặp, đếm từ chính những cuốn sách đã soát.
//!
//! Đây là **quan sát trực tiếp**, không phải suy luận: `khôgn` gặp 153 lần trong
//! một bộ truyện và lần nào cũng là `không`. Mọi tầng khác trong repo đều đang
//! suy — suy từ bàn phím (giá của phép sửa), từ từ điển (từ ghép), từ thống kê
//! của cuốn sách (cụm tên riêng). Nên bảng này xếp trên tất cả.
//!
//! # Vì sao giữ cả mục **nhiều đáp án**
//!
//! Chỗ này là điều quan trọng nhất của cả module. `măt` gặp 20 lần trong hai bộ
//! truyện: 12 lần đúng là `mắt`, 7 lần là `mặt`, 1 lần là `mật`. Không có đáp án
//! nào đúng cho nó ngoài ngữ cảnh.
//!
//! Bảng nào chỉ giữ đáp án đông nhất sẽ sai 8 trong 20 chỗ ấy, và sai **im lặng**
//! — vì nó xếp trên mọi tầng khác nên không tầng nào gỡ được nữa. Nên mục nhiều
//! đáp án không quyết gì cả; nó chỉ **thu hẹp** danh sách ứng viên xuống đúng
//! những chữ đã thật sự gặp, rồi để tầng tên riêng, tầng từ ghép và mô hình chọn
//! tiếp trong đó.
//!
//! Đo trên hai bộ truyện: 97 mục, trong đó 72 mục một đáp án và 25 mục nhiều đáp
//! án. Tức là **một phần tư** số typo hay gặp không tự quyết được — bỏ vế ấy đi
//! là bỏ luôn một phần tư số ca vào chỗ đoán bừa.

use std::collections::HashMap;

/// Bảng nhúng thẳng vào file thực thi, cùng lý do như `de_nham` và `tu_dien`:
/// ứng dụng phải chạy được khi chỉ có mỗi file `.exe`.
const BANG: &str = include_str!("../../../du-lieu/typo.txt");

pub struct Bang(HashMap<String, Vec<String>>);

impl Bang {
    pub fn nap() -> Bang {
        let mut m: HashMap<String, Vec<String>> = HashMap::new();
        for dong in BANG.lines() {
            let d = dong.trim();
            if d.is_empty() || d.starts_with('#') {
                continue;
            }
            let Some((trai, phai)) = d.split_once("=>") else { continue };
            let dap: Vec<String> =
                phai.split('|').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
            if dap.is_empty() {
                continue;
            }
            m.insert(trai.trim().to_lowercase(), dap);
        }
        Bang(m)
    }

    /// Mọi đáp án đã quan sát được cho một typo, nhiều nhất đứng trước.
    ///
    /// `None` nghĩa là bảng chưa từng gặp chữ này — chứ **không** nghĩa là chữ ấy
    /// viết đúng. Bảng dựng từ vài cuốn sách, nó không phủ được tiếng Việt.
    pub fn tra(&self, chu: &str) -> Option<&[String]> {
        self.0.get(&chu.to_lowercase()).map(|v| v.as_slice())
    }

    pub fn so_muc(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod kiem {
    use super::*;

    #[test]
    fn nap_duoc_bang() {
        let b = Bang::nap();
        assert!(b.so_muc() > 50, "bảng quá nhỏ: {}", b.so_muc());
    }

    #[test]
    fn muc_mot_dap_an_va_muc_nhieu_dap_an() {
        let b = Bang::nap();
        // `khôgn` là ca dứt khoát nhất trong cả bảng: 153 lần, lần nào cũng
        // `không`.
        assert_eq!(b.tra("khôgn"), Some(["không".to_string()].as_slice()));
        // Còn `măt` thì không tự quyết được, và bảng phải nói ra điều đó thay vì
        // chọn bừa cái đông nhất.
        let dap = b.tra("măt").expect("thiếu mục `măt`");
        assert!(dap.len() > 1, "{dap:?}");
        assert!(dap.contains(&"mắt".to_string()) && dap.contains(&"mặt".to_string()), "{dap:?}");
    }

    #[test]
    fn khong_phan_biet_hoa_thuong() {
        let b = Bang::nap();
        assert!(b.tra("Khôgn").is_some());
    }
}
