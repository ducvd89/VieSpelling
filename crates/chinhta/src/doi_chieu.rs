//! Đối chiếu đoạn trước và sau khi sửa, ra danh sách khoảng byte cần vá.
//!
//! Vì sao cần bước này thay vì dùng thẳng danh sách phép sửa: các tầng chạy nối
//! tiếp nhau, mỗi tầng nhận chuỗi mà tầng trước đã sửa. Nên vị trí byte trong
//! phép sửa của tầng 4 nói về một chuỗi **không còn tồn tại** — nó không trỏ
//! vào file gốc, cũng chẳng trỏ vào kết quả cuối. Cộng dồn các độ lệch qua từng
//! tầng thì làm được, nhưng chỉ cần một tầng quên báo là lệch lặng lẽ, và lệch
//! vị trí byte nghĩa là cắt vào giữa một chữ khác.
//!
//! So thẳng hai đầu thì không có gì để quên. Danh sách phép sửa vẫn giữ, nhưng
//! chỉ để **viết báo cáo** — nó là chỗ duy nhất biết *vì sao* sửa.
//!
//! Đơn vị đối chiếu là "mảnh" — một cụm chữ liền hoặc một cụm khoảng trắng liền
//! — chứ không phải từng ký tự. Đủ mịn để mỗi lỗi ra một khoảng vá riêng thay
//! vì gộp cả đoạn làm một, mà lại rẻ: đoạn văn cỡ trăm mảnh nên bảng quy hoạch
//! động chỉ vài vạn ô.

use std::ops::Range;

/// Một chỗ khác nhau giữa hai bản.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Khac {
    /// Khoảng byte trong bản **cũ**.
    pub cu: Range<usize>,
    /// Chữ thay vào.
    pub moi: String,
}

/// Cắt chuỗi thành mảnh: xen kẽ cụm khoảng trắng và cụm không phải khoảng trắng.
fn cat_manh(s: &str) -> Vec<Range<usize>> {
    let mut ra = Vec::new();
    let mut dau = 0usize;
    let mut trang: Option<bool> = None;
    for (vt, c) in s.char_indices() {
        let t = c.is_whitespace();
        match trang {
            None => trang = Some(t),
            Some(cu) if cu != t => {
                ra.push(dau..vt);
                dau = vt;
                trang = Some(t);
            }
            _ => {}
        }
    }
    if dau < s.len() {
        ra.push(dau..s.len());
    }
    ra
}

/// So hai bản, trả về các chỗ khác nhau.
pub fn so(cu: &str, moi: &str) -> Vec<Khac> {
    if cu == moi {
        return Vec::new();
    }
    let mc = cat_manh(cu);
    let mm = cat_manh(moi);
    let a: Vec<&str> = mc.iter().map(|r| &cu[r.clone()]).collect();
    let b: Vec<&str> = mm.iter().map(|r| &moi[r.clone()]).collect();

    // Chuỗi con chung dài nhất, kiểu quy hoạch động thẳng. Đoạn văn dài bất
    // thường thì bỏ cuộc và trả về **một** khoảng phủ cả đoạn — vẫn đúng, chỉ
    // là vá thô hơn, và ở đoạn dài thì khả năng nó vắt qua ranh giới thẻ cao
    // hơn nên có thể bị bỏ. Đổi lại là không treo máy.
    if a.len() * b.len() > 4_000_000 {
        return vec![Khac { cu: 0..cu.len(), moi: moi.to_string() }];
    }

    let (n, m) = (a.len(), b.len());
    let mut bang = vec![0u32; (n + 1) * (m + 1)];
    let vt = |i: usize, j: usize| i * (m + 1) + j;
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            bang[vt(i, j)] = if a[i] == b[j] {
                bang[vt(i + 1, j + 1)] + 1
            } else {
                bang[vt(i + 1, j)].max(bang[vt(i, j + 1)])
            };
        }
    }

    // Lần theo bảng, gom các mảnh khác nhau liền kề thành một khoảng vá.
    let mut ra: Vec<Khac> = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    let mut dang_khac: Option<(usize, usize, String)> = None;
    let chot = |dang_khac: &mut Option<(usize, usize, String)>, ra: &mut Vec<Khac>| {
        if let Some((d, c, m)) = dang_khac.take() {
            ra.push(Khac { cu: d..c, moi: m });
        }
    };

    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            chot(&mut dang_khac, &mut ra);
            i += 1;
            j += 1;
        } else if j < m && (i == n || bang[vt(i, j + 1)] >= bang[vt(i + 1, j)]) {
            // Thêm mảnh mới. Neo vào vị trí hiện tại của bản cũ — khoảng rỗng.
            let vtri = if i < n { mc[i].start } else { cu.len() };
            match &mut dang_khac {
                Some((_, c, s)) if *c == vtri => s.push_str(b[j]),
                _ => {
                    chot(&mut dang_khac, &mut ra);
                    dang_khac = Some((vtri, vtri, b[j].to_string()));
                }
            }
            j += 1;
        } else {
            // Bỏ mảnh cũ.
            match &mut dang_khac {
                Some((_, c, _)) if *c == mc[i].start => *c = mc[i].end,
                _ => {
                    chot(&mut dang_khac, &mut ra);
                    dang_khac = Some((mc[i].start, mc[i].end, String::new()));
                }
            }
            i += 1;
        }
    }
    chot(&mut dang_khac, &mut ra);
    ra.retain(|k| !k.cu.is_empty() || !k.moi.is_empty());
    ra
}

#[cfg(test)]
mod kiem {
    use super::*;

    /// Áp lại các khoảng vá để chắc là chúng dựng đúng bản mới.
    fn ap(cu: &str, k: &[Khac]) -> String {
        let mut ra = String::new();
        let mut cuoi = 0usize;
        for x in k {
            ra.push_str(&cu[cuoi..x.cu.start]);
            ra.push_str(&x.moi);
            cuoi = x.cu.end;
        }
        ra.push_str(&cu[cuoi..]);
        ra
    }

    #[test]
    fn khong_doi_thi_khong_co_khoang_nao() {
        assert!(so("một hai ba", "một hai ba").is_empty());
    }

    #[test]
    fn dung_lai_duoc_ban_moi() {
        let ca = [
            ("Anh ấy xử dụng máy", "Anh ấy sử dụng máy"),
            ("một   hai", "một hai"),
            ("nói , rồi đi .", "nói, rồi đi."),
            ("Tình thuơng của mẹ", "Tình thương của mẹ"),
            ("bỏ hết", ""),
            ("", "thêm mới"),
        ];
        for (cu, moi) in ca {
            let k = so(cu, moi);
            assert_eq!(ap(cu, &k), moi, "dựng lại sai: {cu:?} → {moi:?}");
        }
    }

    #[test]
    fn khoang_va_bam_sat_cho_doi() {
        // Một lỗi giữa đoạn dài phải ra **một** khoảng nhỏ, không phải cả đoạn.
        // Khoảng càng rộng càng dễ vắt qua ranh giới thẻ HTML rồi bị bỏ.
        let cu = "Câu này rất dài và chỉ có một chỗ thuơng sai ở giữa mà thôi";
        let moi = "Câu này rất dài và chỉ có một chỗ thương sai ở giữa mà thôi";
        let k = so(cu, moi);
        assert_eq!(k.len(), 1);
        assert_eq!(&cu[k[0].cu.clone()], "thuơng");
        assert_eq!(k[0].moi, "thương");
    }

    #[test]
    fn nhieu_cho_sua_ra_nhieu_khoang() {
        let cu = "Anh xử dụng nó , rồi thuơng tiếc";
        let moi = "Anh sử dụng nó, rồi thương tiếc";
        let k = so(cu, moi);
        assert!(k.len() >= 2, "gộp hết làm một: {k:?}");
        assert_eq!(ap(cu, &k), moi);
    }

    #[test]
    fn khoang_luon_dung_ranh_gioi_ky_tu() {
        let cu = "chữ tiếng Việt thuơng đầy dấu";
        let moi = "chữ tiếng Việt thương đầy dấu";
        for k in so(cu, moi) {
            assert!(cu.is_char_boundary(k.cu.start) && cu.is_char_boundary(k.cu.end));
        }
    }
}
