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
    /// Trả `None` khi khoảng **vắt qua hai nút** — tức phép sửa đè lên ranh
    /// giới thẻ, ví dụ muốn sửa `khô<i>ng</i>` thành `không`. Sửa được ca ấy thì
    /// phải xoá thẻ `<i>`, mà đó là đổi cấu trúc chứ không phải sửa chính tả.
    /// Bỏ qua là đúng, và ca này hiếm.
    pub fn ve_file(&self, r: &Range<usize>) -> Option<Range<usize>> {
        let m = self
            .manh
            .iter()
            .find(|m| r.start >= m.trong_doan.start && r.end <= m.trong_doan.end)?;
        Some(m.ve_file(r.start)?..m.ve_file(r.end)?)
    }
}

/// Thẻ mà nội dung bên trong không phải văn xuôi — bỏ hẳn.
const THE_BO_QUA: [&str; 4] = ["script", "style", "code", "pre"];

/// Thẻ khối: gặp thì ngắt đoạn. `br` cũng ở đây vì nó ngắt dòng thật.
const THE_KHOI: [&str; 27] = [
    "p", "div", "br", "h1", "h2", "h3", "h4", "h5", "h6", "li", "td", "th", "tr", "blockquote",
    "section", "article", "hr", "table", "ul", "ol", "dl", "dd", "dt", "figure", "figcaption",
    "aside", "body",
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
        // "không" nằm vắt qua ranh giới thẻ `<i>` nên không vá được.
        assert!(d[0].ve_file(&(0.."không".len())).is_none());
    }
}
