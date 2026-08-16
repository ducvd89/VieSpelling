//! Đọc và ghi EPUB.
//!
//! Nguyên tắc xuyên suốt: **ghi lại đúng cuốn sách cũ, trừ những byte đã sửa.**
//! EPUB là file zip chứa XHTML, và cả hai tầng đều có cách âm thầm làm hỏng:
//!
//! - Tầng zip: mục `mimetype` **bắt buộc phải là mục đầu tiên và không nén**.
//!   Nén nó lại thì file vẫn giải nén được, `flutter`/`calibre` vẫn mở được, mà
//!   máy đọc sách phần cứng và bộ kiểm epubcheck thì báo hỏng. Ở đây các mục
//!   được chép **nguyên khối đã nén** (`raw_copy_file`) nên giữ đúng cả phương
//!   pháp nén lẫn thứ tự — trừ file ta sửa thì mới nén lại.
//! - Tầng XHTML: xem [`quet`].

pub mod quet;
pub mod thuc_the;

use anyhow::{bail, Context, Result};
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

/// Một cuốn EPUB đã nạp vào bộ nhớ.
///
/// Nạp hết vào RAM chứ không đọc dần: EPUB tiểu thuyết thường 1–5 MB, sách có
/// ảnh thì vài chục — không đáng để làm cho phức tạp. Đổi lại, file gốc được
/// đóng ngay sau khi nạp nên **ghi đè lên chính nó cũng an toàn**.
pub struct Epub {
    /// Mọi mục trong zip, đúng thứ tự gốc.
    pub muc: Vec<Muc>,
}

pub struct Muc {
    pub ten: String,
    pub noi_dung: Vec<u8>,
}

impl Epub {
    pub fn nap(duong_dan: &Path) -> Result<Epub> {
        let byte = std::fs::read(duong_dan)
            .with_context(|| format!("không đọc được {}", duong_dan.display()))?;
        Epub::nap_tu_byte(byte)
    }

    pub fn nap_tu_byte(byte: Vec<u8>) -> Result<Epub> {
        let mut kho = zip::ZipArchive::new(Cursor::new(byte)).context("không phải file zip hợp lệ")?;
        let mut muc = Vec::with_capacity(kho.len());
        for i in 0..kho.len() {
            let mut f = kho.by_index(i)?;
            if f.is_dir() {
                continue;
            }
            let ten = f.name().to_string();
            let mut noi_dung = Vec::with_capacity(f.size() as usize);
            f.read_to_end(&mut noi_dung)?;
            muc.push(Muc { ten, noi_dung });
        }
        if muc.is_empty() {
            bail!("file zip rỗng, không phải EPUB");
        }
        Ok(Epub { muc })
    }

    /// Các mục là văn bản sách — chỗ duy nhất được sửa.
    ///
    /// Nhận theo đuôi file thay vì đọc `media-type` trong OPF. Đọc OPF thì đúng
    /// bài hơn, nhưng EPUB ngoài đời khai sai media-type nhiều hơn là đặt sai
    /// đuôi file, mà khai sai thì cả chương bị bỏ sót lặng lẽ.
    pub fn chi_so_van_ban(&self) -> Vec<usize> {
        self.muc
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                let t = m.ten.to_ascii_lowercase();
                (t.ends_with(".xhtml") || t.ends_with(".html") || t.ends_with(".htm"))
                    && !t.contains("/toc.")
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Nhan đề lấy từ OPF, chỉ để hiện trong báo cáo.
    pub fn nhan_de(&self) -> Option<String> {
        let opf = self.muc.iter().find(|m| m.ten.to_ascii_lowercase().ends_with(".opf"))?;
        let s = String::from_utf8_lossy(&opf.noi_dung);
        let dau = s.find("<dc:title").or_else(|| s.find("<title"))?;
        let mo = dau + s[dau..].find('>')? + 1;
        let dong = mo + s[mo..].find("</")?;
        let t = thuc_the::giai_ma(s[mo..dong].trim());
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    }

    /// Ghi ra file mới.
    ///
    /// `da_sua` là tập chỉ số mục đã đổi nội dung; những mục còn lại được chép
    /// thẳng khối nén cũ sang, không giải nén rồi nén lại.
    pub fn ghi(&self, goc: &Path, ra: &Path, da_sua: &[usize]) -> Result<()> {
        let byte = std::fs::read(goc)?;
        let mut kho = zip::ZipArchive::new(Cursor::new(byte))?;
        let f = std::fs::File::create(ra).with_context(|| format!("không tạo được {}", ra.display()))?;
        let mut viet = zip::ZipWriter::new(f);

        // Đối chiếu theo **tên** chứ không theo chỉ số: `nap` bỏ các mục thư mục
        // nên chỉ số hai bên đã lệch nhau.
        let ten_da_sua: Vec<&str> = da_sua.iter().map(|&i| self.muc[i].ten.as_str()).collect();
        for i in 0..kho.len() {
            let cu = kho.by_index_raw(i)?;
            let ten = cu.name().to_string();
            if cu.is_dir() {
                drop(cu);
                continue;
            }
            match ten_da_sua.iter().position(|&t| t == ten) {
                None => {
                    // Chép nguyên khối đã nén. Giữ được cả `mimetype` không nén
                    // đứng đầu lẫn mọi thứ ta không đụng tới.
                    viet.raw_copy_file(cu)?;
                }
                Some(k) => {
                    let chi_so = da_sua[k];
                    drop(cu);
                    let tuy = zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated);
                    viet.start_file(&ten, tuy)?;
                    viet.write_all(&self.muc[chi_so].noi_dung)?;
                }
            }
        }
        viet.finish()?;
        Ok(())
    }
}

/// Đọc nội dung một mục thành chuỗi UTF-8, bỏ dấu thứ tự byte nếu có.
///
/// Trả `None` khi file không phải UTF-8 hợp lệ. EPUB bắt buộc UTF-8 nên ca này
/// nghĩa là file hỏng sẵn — bỏ qua chứ không đoán bảng mã, vì đoán sai thì ghi
/// lại là hỏng thật.
pub fn doc_chuoi(byte: &[u8]) -> Option<String> {
    let byte = byte.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(byte);
    String::from_utf8(byte.to_vec()).ok()
}

/// Chép mọi thứ vào một zip mới — dùng khi cần ghi mà không còn file gốc.
pub fn ghi_moi<W: Write + Seek>(w: W, muc: &[Muc]) -> Result<()> {
    let mut viet = zip::ZipWriter::new(w);
    for (i, m) in muc.iter().enumerate() {
        // `mimetype` phải đứng đầu và **không nén** — đây là chỗ duy nhất trong
        // đặc tả EPUB bắt buộc một phương pháp nén cụ thể.
        let tuy = if i == 0 && m.ten == "mimetype" {
            zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)
        } else {
            zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)
        };
        viet.start_file(&m.ten, tuy)?;
        viet.write_all(&m.noi_dung)?;
    }
    viet.finish()?;
    Ok(())
}
