//! Quét một file XHTML thành các **đoạn văn bản đọc được**, kèm đường về.
//!
//! Đây là chỗ quyết định cả ứng dụng có làm hỏng sách hay không, nên nói rõ hai
//! lựa chọn thiết kế:
//!
//! **Không dựng cây DOM rồi in lại.** In lại cây bao giờ cũng khác bản gốc —
//! thứ tự thuộc tính, kiểu đóng thẻ rỗng, khoảng trắng trong thẻ, khai báo
//! namespace. Với một cuốn sách hàng trăm file thì đó là hàng vạn thay đổi
//! không ai yêu cầu. Ở đây chỉ **vá từng khoảng byte** đúng chỗ chữ cần sửa;
//! mọi byte khác giữ nguyên si.
//!
//! **Nối các nút văn bản trong cùng một khối.** `khô<i>ng</i>` nằm trong file
//! thành hai nút. Kiểm riêng từng nút thì `khô` và `ng` đều không phải tiếng
//! hợp lệ, và bộ dò báo hai lỗi ma ngay giữa một chữ vốn đúng. Nên các nút được
//! nối lại thành một chuỗi liền theo khối, dò trên chuỗi ấy, rồi ánh xạ ngược
//! về đúng nút và đúng byte.

use crate::thuc_the;
use std::ops::Range;

/// Một mảnh văn bản liền mạch trong file — đúng một nút văn bản XHTML.
#[derive(Debug, Clone)]
pub struct Manh {
    /// Vị trí trong chuỗi [`Doan::chu`].
    pub trong_doan: Range<usize>,
    /// Vị trí byte của mảnh thô trong file.
    pub trong_file: Range<usize>,
    /// Bản đồ vị-trí-đã-giải-mã → vị-trí-thô. `None` nghĩa là trùng khít.
    ban_do: Option<Vec<u32>>,
}

impl Manh {
    /// Đổi một vị trí trong đoạn thành vị trí byte trong file.
    pub fn ve_file(&self, trong_doan: usize) -> Option<usize> {
        let cuc_bo = trong_doan.checked_sub(self.trong_doan.start)?;
        if trong_doan > self.trong_doan.end {
            return None;
        }
        match &self.ban_do {
            None => Some(self.trong_file.start + cuc_bo),
            Some(bd) => bd.get(cuc_bo).map(|&v| self.trong_file.start + v as usize),
        }
    }
}

/// Một khối văn bản đọc liền — thường là một thẻ `<p>`.
#[derive(Debug, Clone)]
pub struct Doan {
    /// Chữ đã giải mã thực thể và nối liền các nút.
    pub chu: String,
    pub manh: Vec<Manh>,
}

impl Doan {
    /// Đổi một khoảng trong đoạn thành khoảng byte trong file.
    ///
    /// Trả `None` khi khoảng **vắt qua hai nút**, tức phép sửa đè lên ranh giới
    /// thẻ. Dùng [`Doan::ve_file_qua_the`] để vá được cả ca ấy.
    pub fn ve_file(&self, r: &Range<usize>) -> Option<Range<usize>> {
        let m = self
            .manh
            .iter()
            .find(|m| r.start >= m.trong_doan.start && r.end <= m.trong_doan.end)?;
        Some(m.ve_file(r.start)?..m.ve_file(r.end)?)
    }

    /// Như [`Doan::ve_file`] nhưng vá được cả chỗ **vắt qua thẻ định dạng**.
    ///
    /// Sách convert hay cắt chữ làm đôi bằng một thẻ vô nghĩa —
    /// `thuơ<i>ng</i>`, `khô<b>ng</b>`. [`quet`] đã nối chúng lại nên bộ dò
    /// **đọc** đúng chữ, nhưng lúc **vá** thì chỗ sửa nằm trên hai nút và
    /// `ve_file` đành bỏ. Kết quả là một lớp lỗi được tìm ra rồi bị bỏ rơi.
    ///
    /// Trả về các khoảng byte mà chỗ sửa phủ lên, theo thứ tự. Người gọi ghi
    /// toàn bộ chữ mới vào khoảng **đầu tiên** và xoá rỗng những khoảng sau.
    /// Thẻ vẫn nằm nguyên chỗ cũ, chỉ là rỗng ruột — `thương<i></i>`. Trình đọc
    /// nào cũng hiển thị y hệt, và phần định dạng bị mất vốn là định dạng của
    /// nửa chữ, tức là đằng nào cũng vô nghĩa.
    ///
    /// Trả `None` khi giữa hai nút có thứ gì **không phải thẻ định dạng** —
    /// ảnh, liên kết, ngắt dòng, chú thích. Ở đó việc dồn chữ về một phía làm
    /// đổi thứ tự nội dung, không còn là sửa chính tả nữa.
    pub fn ve_file_qua_the(&self, file: &str, r: &Range<usize>) -> Option<Vec<Range<usize>>> {
        if let Some(mot) = self.ve_file(r) {
            return Some(vec![mot]);
        }
        let phu: Vec<&Manh> = self
            .manh
            .iter()
            .filter(|m| m.trong_doan.start < r.end && r.start < m.trong_doan.end)
            .collect();
        if phu.len() < 2 {
            return None;
        }
        for hai in phu.windows(2) {
            let giua = file.get(hai[0].trong_file.end..hai[1].trong_file.start)?;
            if !chi_toan_the_dinh_dang(giua) {
                return None;
            }
        }
        let mut ra = Vec::with_capacity(phu.len());
        for m in phu {
            let dau = r.start.max(m.trong_doan.start);
            let cuoi = r.end.min(m.trong_doan.end);
            ra.push(m.ve_file(dau)?..m.ve_file(cuoi)?);
        }
        Some(ra)
    }
}

/// Thẻ chỉ đổi **cách hiển thị chữ**, không đổi nội dung hay thứ tự của nó.
///
/// Danh sách cố tình hẹp. Mọi thẻ ngoài đây — `a`, `img`, `br`, `ruby`, `sup`
/// dùng làm chú thích — đều mang thông tin riêng, và dồn chữ qua chúng là đổi
/// nghĩa chứ không phải sửa chính tả.
const THE_DINH_DANG: [&str; 9] =
    ["i", "b", "em", "strong", "span", "u", "s", "small", "big"];

/// Khoảng giữa hai nút văn bản có chỉ gồm thẻ định dạng không.
fn chi_toan_the_dinh_dang(giua: &str) -> bool {
    let b = giua.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        if b[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if b[i] != b'<' {
            return false;
        }
        let het = het_the(giua, i);
        let (ten, _) = doc_ten(&giua[i..het]);
        if !THE_DINH_DANG.contains(&ten.as_str()) {
            return false;
        }
        i = het;
    }
    true
}

/// Thẻ mà nội dung bên trong không phải văn xuôi — bỏ hẳn.
const THE_BO_QUA: [&str; 4] = ["script", "style", "code", "pre"];

/// Thẻ khối: gặp thì ngắt đoạn. `br` cũng ở đây vì nó ngắt dòng thật.
///
/// Nhóm cuối (`html`, `head`, `title`, `meta`, `link`) không phải thẻ khối theo
/// nghĩa trình bày, nhưng phải ngắt đoạn ở đó vì lý do khác: không ngắt thì
/// khoảng trắng xuống dòng giữa `</head>` và `<body>` dính vào cùng một đoạn
/// với nhan đề trong `<title>`, rồi tầng dọn khoảng trắng đòi gộp chúng lại —
/// một phép sửa vắt qua `<title>`, tức là vắt qua thẻ không phải định dạng nên
/// không vá được. Đo trên một bộ truyện 2.998 chương thì đó đúng là **5.996**
/// chỗ báo "vướng thẻ HTML", tức hai chỗ mỗi file, mà không chỗ nào là văn xuôi.
///
/// Ngắt ở đây còn được thêm một thứ: nhan đề trong `<title>` thành một đoạn
/// riêng và được kiểm chính tả như mọi đoạn khác.
const THE_KHOI: [&str; 32] = [
    "p", "div", "br", "h1", "h2", "h3", "h4", "h5", "h6", "li", "td", "th", "tr", "blockquote",
    "section", "article", "hr", "table", "ul", "ol", "dl", "dd", "dt", "figure", "figcaption",
    "aside", "body", "html", "head", "title", "meta", "link",
];

/// Quét toàn bộ file, trả về các đoạn.
pub fn quet(file: &str) -> Vec<Doan> {
    let b = file.as_bytes();
    let mut ra: Vec<Doan> = Vec::new();
    let mut chu = String::new();
    let mut manh: Vec<Manh> = Vec::new();
    let mut bo_qua = 0usize; // độ sâu đang nằm trong thẻ bỏ qua
    let mut i = 0usize;

    macro_rules! ngat_doan {
        () => {
            if !chu.trim().is_empty() {
                ra.push(Doan { chu: std::mem::take(&mut chu), manh: std::mem::take(&mut manh) });
            } else {
                chu.clear();
                manh.clear();
            }
        };
    }

    while i < b.len() {
        if b[i] != b'<' {
            let dau = i;
            while i < b.len() && b[i] != b'<' {
                i += 1;
            }
            if bo_qua == 0 {
                let (giai, ban_do) = thuc_the::giai_ma_co_ban_do(&file[dau..i]);
                if !giai.is_empty() {
                    let d = chu.len();
                    chu.push_str(&giai);
                    manh.push(Manh {
                        trong_doan: d..chu.len(),
                        trong_file: dau..i,
                        ban_do,
                    });
                }
            }
            continue;
        }

        // Các dạng không phải thẻ phần tử.
        if file[i..].starts_with("<!--") {
            i = tim_sau(file, i, "-->");
            continue;
        }
        if file[i..].starts_with("<![CDATA[") {
            i = tim_sau(file, i, "]]>");
            continue;
        }
        if file[i..].starts_with("<?") {
            i = tim_sau(file, i, "?>");
            continue;
        }
        if file[i..].starts_with("<!") {
            i = tim_sau(file, i, ">");
            continue;
        }

        let het = het_the(file, i);
        let (ten, dong) = doc_ten(&file[i..het]);
        let tu_dong = file[..het].ends_with("/>");

        if THE_BO_QUA.contains(&ten.as_str()) && !tu_dong {
            if dong {
                bo_qua = bo_qua.saturating_sub(1);
            } else {
                bo_qua += 1;
            }
        }
        if THE_KHOI.contains(&ten.as_str()) {
            ngat_doan!();
        }
        i = het;
    }
    ngat_doan!();
    ra
}

/// Vị trí ngay sau lần xuất hiện đầu của `moc`, hoặc hết file.
fn tim_sau(file: &str, tu: usize, moc: &str) -> usize {
    file[tu..].find(moc).map(|k| tu + k + moc.len()).unwrap_or(file.len())
}

/// Vị trí ngay sau dấu `>` đóng thẻ, **tôn trọng dấu nháy** — giá trị thuộc
/// tính chứa `>` là chuyện có thật (`alt="a > b"`), cắt bừa ở dấu `>` đầu tiên
/// thì phần còn lại của thẻ bị đọc thành văn bản.
fn het_the(file: &str, tu: usize) -> usize {
    let b = file.as_bytes();
    let mut i = tu + 1;
    let mut nhay = 0u8;
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' if nhay == 0 => nhay = b[i],
            k if k == nhay => nhay = 0,
            b'>' if nhay == 0 => return i + 1,
            _ => {}
        }
        i += 1;
    }
    b.len()
}

/// Tên thẻ viết thường, và thẻ này là thẻ đóng hay không.
fn doc_ten(the: &str) -> (String, bool) {
    let than = the.trim_start_matches('<').trim_end_matches('>');
    let (than, dong) = match than.strip_prefix('/') {
        Some(t) => (t, true),
        None => (than, false),
    };
    let ten: String = than
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == ':')
        .collect();
    // Bỏ tiền tố namespace (`epub:switch`) — ta chỉ quan tâm tên cục bộ.
    let ten = ten.rsplit(':').next().unwrap_or("").to_ascii_lowercase();
    (ten, dong)
}

#[cfg(test)]
mod kiem {
    use super::*;

    #[test]
    fn noi_lai_nut_bi_the_inline_cat_vun() {
        let f = "<p>khô<i>ng</i> bao giờ</p>";
        let d = quet(f);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].chu, "không bao giờ");
    }

    #[test]
    fn the_khoi_ngat_doan() {
        let d = quet("<p>một</p><p>hai</p>");
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].chu, "một");
        assert_eq!(d[1].chu, "hai");
    }

    #[test]
    fn bo_qua_script_va_style() {
        let d = quet("<style>p{color:red}</style><p>chữ</p><script>var a=1;</script>");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].chu, "chữ");
    }

    #[test]
    fn vi_tri_ve_file_dung() {
        let f = "<p>một hai ba</p>";
        let d = quet(f);
        let doan = &d[0];
        let vt = doan.chu.find("hai").unwrap();
        let r = doan.ve_file(&(vt..vt + 3)).unwrap();
        assert_eq!(&f[r], "hai");
    }

    #[test]
    fn vi_tri_ve_file_dung_khi_co_thuc_the() {
        // `&amp;` dài 5 byte mà giải ra 1 — mọi vị trí sau nó lệch 4 byte nếu
        // không có bản đồ. Đây chính là ca bản đồ sinh ra để chặn.
        let f = "<p>Tom &amp; Jerry hòa</p>";
        let d = quet(f);
        let doan = &d[0];
        assert_eq!(doan.chu, "Tom & Jerry hòa");
        let vt = doan.chu.find("hòa").unwrap();
        let r = doan.ve_file(&(vt..vt + "hòa".len())).unwrap();
        assert_eq!(&f[r], "hòa");
    }

    #[test]
    fn thuoc_tinh_chua_dau_lon_hon() {
        let d = quet(r#"<p><img alt="a > b"/>chữ</p>"#);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].chu, "chữ");
    }

    #[test]
    fn sua_vat_qua_hai_nut_thi_bo_qua() {
        let f = "<p>khô<i>ng</i></p>";
        let d = quet(f);
        // "không" nằm vắt qua ranh giới thẻ `<i>` nên `ve_file` không vá được.
        assert!(d[0].ve_file(&(0.."không".len())).is_none());
    }

    /// Áp các khoảng vá theo đúng cách `xu_ly` áp: chữ mới vào khoảng đầu, các
    /// khoảng sau xoá rỗng.
    fn va(file: &str, khoang: &[Range<usize>], moi: &str) -> String {
        let mut ra = String::new();
        let mut cuoi = 0usize;
        for (k, r) in khoang.iter().enumerate() {
            ra.push_str(&file[cuoi..r.start]);
            if k == 0 {
                ra.push_str(moi);
            }
            cuoi = r.end;
        }
        ra.push_str(&file[cuoi..]);
        ra
    }

    #[test]
    fn va_duoc_chu_bi_the_dinh_dang_cat_doi() {
        // Sách convert hay cắt chữ làm đôi bằng một thẻ vô nghĩa. Bộ dò đọc
        // đúng `thuơng` (nhờ nối nút), nên nó cũng phải **sửa** được.
        let f = "<p>tình thuơ<i>ng</i> của mẹ</p>";
        let d = quet(f);
        let vt = d[0].chu.find("thuơng").unwrap();
        let khoang = d[0].ve_file_qua_the(f, &(vt..vt + "thuơng".len())).unwrap();
        assert_eq!(khoang.len(), 2, "phải phủ hai nút");
        assert_eq!(va(f, &khoang, "thương"), "<p>tình thương<i></i> của mẹ</p>");
    }

    #[test]
    fn khong_va_qua_the_khong_phai_dinh_dang() {
        // Ảnh, liên kết, ngắt dòng mang thông tin riêng; dồn chữ qua chúng là
        // đổi thứ tự nội dung chứ không phải sửa chính tả.
        for f in [
            r#"<p>thuơ<img src="a.png"/>ng</p>"#,
            r#"<p>thuơ<a href="x">ng</a></p>"#,
            "<p>thuơ<br/>ng</p>",
        ] {
            let d = quet(f);
            let Some(doan) = d.first() else { continue };
            let Some(vt) = doan.chu.find("thuơng") else { continue };
            assert!(
                doan.ve_file_qua_the(f, &(vt..vt + "thuơng".len())).is_none(),
                "không được vá qua: {f}"
            );
        }
    }

    #[test]
    fn va_duoc_qua_nhieu_the_lien_tiep() {
        let f = "<p>thu</b><i>ơng</i></p>";
        let d = quet(f);
        let vt = d[0].chu.find("thuơng").unwrap();
        let khoang = d[0].ve_file_qua_the(f, &(vt..vt + "thuơng".len())).unwrap();
        assert_eq!(va(f, &khoang, "thương"), "<p>thương</b><i></i></p>");
    }
}
