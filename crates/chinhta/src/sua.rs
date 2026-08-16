//! Kiểu chung cho mọi phép sửa.
//!
//! Mọi tầng dò lỗi đều trả về cùng một thứ: **danh sách khoảng byte cần thay**,
//! chứ không trả về văn bản đã sửa. Lý do là tầng EPUB phải ghép các phép sửa
//! trở lại vào file gốc theo kiểu **vá từng khoảng byte**, giữ nguyên mọi byte
//! không đụng tới — thẻ HTML, thuộc tính, thực thể, thứ tự khoảng trắng trong
//! thẻ. Nếu tầng dò trả về chuỗi đã sửa thì phải so hai chuỗi để tìm lại chỗ
//! đổi, mà phép so ấy đoán sai chỗ là hỏng markup.
//!
//! Kèm theo là **lý do** dạng chữ. Ứng dụng tự sửa rồi mới báo cáo, nên báo cáo
//! là thứ duy nhất người dùng có để kiểm lại — một dòng "đã đổi X thành Y" mà
//! không nói vì sao thì không kiểm được.

use std::ops::Range;

/// Lỗi thuộc loại gì. Báo cáo gom nhóm theo đây, và người dùng tắt/bật được
/// từng loại một.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub enum Loai {
    /// Dựng lại Unicode: tổ hợp NFC, bỏ ký tự vô hình.
    Unicode,
    /// Văn bản còn nằm ở bảng mã cũ (TCVN3, VNI).
    BangMa,
    /// Khoảng trắng thừa, thiếu, hoặc sai loại.
    KhoangTrang,
    /// Dấu câu: khoảng trắng quanh dấu, dấu nháy, dấu ba chấm.
    DauCau,
    /// Dấu thanh đặt sai nguyên âm — `qúy`, `ngừơi`, `đựơc`. Sai thật.
    DauThanh,
    /// Kéo `hoà` về `hòa` (hoặc ngược lại) cho khớp phần còn lại của sách.
    ///
    /// Tách hẳn khỏi [`Loai::DauThanh`] vì **đây không phải lỗi**: hai kiểu đặt
    /// dấu đều được công nhận. Gộp chung thì con số "lỗi chữ nghĩa" trong báo
    /// cáo phồng lên hàng nghìn — đo trên một bộ truyện dài thì 7.215 trong số
    /// 7.731 "lỗi" hoá ra chỉ là đổi kiểu trình bày, và người đọc báo cáo tưởng
    /// bản dịch sai chính tả tràn lan.
    KieuDau,
    /// Tiếng sai cấu tạo — không ghép được từ âm đầu + vần + thanh nào.
    AmTietSai,
    /// Cặp dễ nhầm, phải nhìn ngữ cảnh mới phân được.
    DeNham,
    /// Hai tiếng dính liền vì mất khoảng trắng: `Phúlần` → `Phú lần`.
    ///
    /// Tách khỏi [`Loai::AmTietSai`] vì bằng chứng khác hẳn về chất: ở đây
    /// **không chữ cái nào sai**, chỉ thiếu một khoảng trắng. Gộp chung thì báo
    /// cáo không cho người đọc thấy sự khác biệt ấy, mà nó là khác biệt giữa
    /// "máy giữ nguyên từng ký tự bạn gõ" và "máy đoán bạn định gõ gì".
    DinhChu,
}

impl Loai {
    pub fn ten(self) -> &'static str {
        match self {
            Loai::Unicode => "Unicode",
            Loai::BangMa => "Bảng mã cũ",
            Loai::KhoangTrang => "Khoảng trắng",
            Loai::DauCau => "Dấu câu",
            Loai::DauThanh => "Dấu thanh",
            Loai::KieuDau => "Kiểu đặt dấu",
            Loai::AmTietSai => "Tiếng sai",
            Loai::DeNham => "Dễ nhầm",
            Loai::DinhChu => "Dính chữ",
        }
    }
}

/// Mức chắc chắn. Quyết định phép sửa có được tự áp hay không.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum DoChac {
    /// Sai theo luật, không cần nhìn ngữ cảnh: `qúy`, hai khoảng trắng liền,
    /// khoảng trắng trước dấu phẩy. Luôn tự sửa.
    Chac,
    /// Suy ra từ cấu tạo và chỉ có **một** cách sửa hợp lý. Tự sửa được.
    KhaChac,
    /// Phải nhìn ngữ cảnh mới biết. Chỉ tự sửa khi mô hình ngôn ngữ chấm điểm
    /// cách sửa hơn hẳn bản gốc.
    NgoVuc,
}

/// Một phép sửa: thay `pham_vi` trong văn bản gốc bằng `thay_bang`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SuaDoi {
    /// Khoảng **byte** trong văn bản đưa vào. Luôn rơi đúng ranh giới ký tự.
    #[serde(skip)]
    pub pham_vi: Range<usize>,
    pub goc: String,
    pub thay_bang: String,
    pub loai: Loai,
    pub do_chac: DoChac,
    pub ly_do: String,
}

impl SuaDoi {
    pub fn moi(
        pham_vi: Range<usize>,
        goc: impl Into<String>,
        thay_bang: impl Into<String>,
        loai: Loai,
        do_chac: DoChac,
        ly_do: impl Into<String>,
    ) -> Self {
        SuaDoi {
            pham_vi,
            goc: goc.into(),
            thay_bang: thay_bang.into(),
            loai,
            do_chac,
            ly_do: ly_do.into(),
        }
    }

    /// Phép sửa này có thật sự đổi gì không. Các tầng dò sinh ra khá nhiều phép
    /// sửa rỗng (chuẩn hoá xong ra đúng chuỗi cũ), lọc ở một chỗ cho gọn.
    pub fn co_doi(&self) -> bool {
        self.goc != self.thay_bang
    }
}

/// Áp một danh sách phép sửa vào văn bản.
///
/// Danh sách **không cần** sắp sẵn, nhưng các khoảng **không được chồng nhau** —
/// chồng nhau thì phép sau bị bỏ, vì áp cả hai lên cùng một chỗ cho ra chuỗi
/// tuỳ thứ tự, tức là kết quả không xác định. Trả về số phép thật sự áp được.
pub fn ap_dung(goc: &str, sua: &mut [SuaDoi]) -> (String, usize) {
    sua.sort_by_key(|s| (s.pham_vi.start, s.pham_vi.end));
    let mut ra = String::with_capacity(goc.len());
    let mut cuoi = 0usize;
    let mut dem = 0usize;
    for s in sua.iter() {
        // Bỏ phép chồng lên phép trước, trỏ ra ngoài văn bản, hoặc rơi vào giữa
        // một ký tự nhiều byte. Ca cuối là thứ dễ sinh ra nhất khi thêm tầng dò
        // mới, và nó **panic** chứ không sai lặng lẽ — chắn ở đây để một tầng
        // dò lỗi không kéo sập cả lượt xử lý sách.
        if s.pham_vi.start < cuoi
            || s.pham_vi.end > goc.len()
            || !goc.is_char_boundary(s.pham_vi.start)
            || !goc.is_char_boundary(s.pham_vi.end)
        {
            continue;
        }
        ra.push_str(&goc[cuoi..s.pham_vi.start]);
        ra.push_str(&s.thay_bang);
        cuoi = s.pham_vi.end;
        dem += 1;
    }
    ra.push_str(&goc[cuoi..]);
    (ra, dem)
}

#[cfg(test)]
mod kiem {
    use super::*;

    fn sd(r: Range<usize>, goc: &str, moi: &str) -> SuaDoi {
        SuaDoi::moi(r, goc, moi, Loai::KhoangTrang, DoChac::Chac, "kiểm")
    }

    #[test]
    fn ap_theo_thu_tu_bat_ky() {
        let goc = "một hai ba";
        let mut s = vec![sd(10..12, "ba", "BA"), sd(0..5, "một", "MỘT")];
        let (ra, n) = ap_dung(goc, &mut s);
        assert_eq!(ra, "MỘT hai BA");
        assert_eq!(n, 2);
    }

    #[test]
    fn bo_qua_pham_vi_chong_nhau() {
        // Hai tầng cùng đòi sửa một chỗ. Áp cả hai thì kết quả tuỳ thứ tự sắp,
        // nên chỉ giữ phép đầu và đếm lại cho báo cáo khớp thực tế.
        let goc = "abc";
        let mut s = vec![sd(0..2, "ab", "X"), sd(1..3, "bc", "Y")];
        let (ra, n) = ap_dung(goc, &mut s);
        assert_eq!(ra, "Xc");
        assert_eq!(n, 1);
    }
}
