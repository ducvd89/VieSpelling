//! Bài kiểm đầu-cuối: dựng một EPUB có lỗi cài sẵn, chạy cả đường đi, rồi mở
//! file ra kiểm lại.
//!
//! Bài kiểm đơn vị ở từng crate không bắt được lớp lỗi nguy hiểm nhất của ứng
//! dụng này — **làm hỏng file**. Chữ sửa đúng mà zip dựng sai, hoặc thẻ HTML bị
//! xén, thì mọi bài kiểm kia vẫn xanh còn cuốn sách thì không mở được.

use chinhta::soat::TuyChon;
use sach::{Epub, Muc};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use ungdung::nhat_ky::{Bao, Tin};
use ungdung::xu_ly;

const CONTAINER: &str = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
<rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#;

const OPF: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id">
<metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:title>Sách thử nghiệm</dc:title><dc:identifier id="id">thu-nghiem</dc:identifier>
<dc:language>vi</dc:language></metadata>
<manifest><item id="c1" href="ch1.xhtml" media-type="application/xhtml+xml"/></manifest>
<spine><itemref idref="c1"/></spine></package>"#;

/// Chương có lỗi cài sẵn. Mỗi đoạn nhắm một tầng dò khác nhau.
fn chuong() -> String {
    let mut s = String::from(
        r#"<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml"><head><title>Chương một</title>
<style>p { margin: 0 }</style></head><body>
"#,
    );
    // Cặp dễ nhầm, dạng luôn sai.
    s.push_str("<p>Anh ấy xử dụng máy tính để làm việc.</p>\n");
    // Dấu thanh đặt sai sau `qu`.
    s.push_str("<p>Một món quà rất qúy giá.</p>\n");
    // Khoảng trắng trước dấu phẩy, khoảng trắng lặp, thiếu trắng sau dấu chấm.
    s.push_str("<p>Trời mưa , rồi   tạnh.Anh đi ra.</p>\n");
    // Gạch nối mềm vô hình nằm giữa chữ.
    s.push_str("<p>Trời khô\u{00AD}ng mưa nữa.</p>\n");
    // Tiếng sai cấu tạo mà hàng xóm quyết được: từ điển có `tình thương`, không
    // có `tình thường`. Sửa được ngay, không cần mô hình.
    s.push_str("<p>Tình thuơng của mẹ thật lớn.</p>\n");
    // Tiếng sai cấu tạo mà **không** hàng xóm nào quyết được — phải để ngỏ.
    s.push_str("<p>Ừ thuơng à.</p>\n");
    // Không được đụng: số kiểu Việt Nam, tên riêng nước ngoài, thực thể XML.
    s.push_str("<p>Giá 1,5 triệu và 12.000 đồng lúc 10:30.</p>\n");
    s.push_str("<p>Giáo sư Dumbledore nhìn Voldemort &amp; bỏ đi.</p>\n");
    // Chữ bị thẻ inline cắt đôi — phải nối lại để khỏi báo lỗi ma.
    s.push_str("<p>Trời khô<i>ng</i> bao giờ đổi.</p>\n");
    // Hai khoảng trắng nằm vắt qua thẻ đóng: nút trước kết thúc bằng khoảng
    // trắng, nút sau bắt đầu bằng khoảng trắng. Gộp chúng thì phải xén thẻ.
    s.push_str("<p>Xin <i>chào </i> bạn nhé.</p>\n");
    // Trong `<pre>` thì không đụng tới.
    s.push_str("<pre>xử dụng</pre>\n");
    s.push_str("</body></html>\n");
    s
}

fn dung_epub(tai: &Path) {
    let muc = vec![
        Muc { ten: "mimetype".into(), noi_dung: b"application/epub+zip".to_vec() },
        Muc { ten: "META-INF/container.xml".into(), noi_dung: CONTAINER.as_bytes().to_vec() },
        Muc { ten: "OEBPS/content.opf".into(), noi_dung: OPF.as_bytes().to_vec() },
        Muc { ten: "OEBPS/ch1.xhtml".into(), noi_dung: chuong().into_bytes() },
    ];
    let f = std::fs::File::create(tai).unwrap();
    sach::ghi_moi(f, &muc).unwrap();
}

/// Chạy đường xử lý, nuốt hết nhật ký.
///
/// Bài kiểm không quan tâm nhật ký, nhưng vẫn phải dựng một [`Bao`] thật để đi
/// qua đúng đường mà ứng dụng đi — nuốt nhật ký ở đây chứ không thêm một nhánh
/// "chạy không báo cáo" vào lõi, vì nhánh ấy sẽ là nhánh duy nhất được kiểm.
fn chay_xu_ly(vao: &Path, ra: &Path) -> xu_ly::KetQuaSach {
    let mut nuot = |_: Tin| {};
    let mut bao = Bao::moi(&mut nuot);
    xu_ly::xu_ly(vao, ra, TuyChon::default(), None, &mut bao).unwrap()
}

fn cho_tam(ten: &str) -> PathBuf {
    let d = std::env::temp_dir().join("vsc-kiem");
    std::fs::create_dir_all(&d).unwrap();
    d.join(ten)
}

/// Chạy cả đường đi, trả về (kết quả, nội dung chương sau khi sửa).
fn chay(ten: &str) -> (xu_ly::KetQuaSach, String) {
    let vao = cho_tam(&format!("{ten}-vao.epub"));
    let ra = cho_tam(&format!("{ten}-ra.epub"));
    dung_epub(&vao);
    let kq = chay_xu_ly(&vao, &ra);
    let epub = Epub::nap(&ra).unwrap();
    let ch = epub.muc.iter().find(|m| m.ten.ends_with("ch1.xhtml")).unwrap();
    (kq, String::from_utf8(ch.noi_dung.clone()).unwrap())
}

#[test]
fn sua_dung_nhung_loi_cai_san() {
    let (kq, ch) = chay("sua");
    assert!(ch.contains("sử dụng máy tính"), "chưa sửa cặp dễ nhầm:\n{ch}");
    assert!(ch.contains("rất quý giá"), "chưa sửa dấu thanh sau `qu`:\n{ch}");
    assert!(ch.contains("Trời mưa, rồi tạnh. Anh đi ra."), "chưa dọn dấu câu:\n{ch}");
    assert!(ch.contains("Trời không mưa nữa"), "chưa bỏ gạch nối mềm:\n{ch}");
    assert!(kq.da_sua.len() >= 5, "quá ít phép sửa: {:?}", kq.da_sua);
}

#[test]
fn khong_dung_vao_thu_khong_duoc_dung() {
    let (_, ch) = chay("giu");
    // Số kiểu Việt Nam: dấu phẩy thập phân, dấu chấm hàng nghìn, dấu hai chấm giờ.
    assert!(ch.contains("Giá 1,5 triệu và 12.000 đồng lúc 10:30."), "đã phá con số:\n{ch}");
    // Tên riêng nước ngoài.
    assert!(ch.contains("Dumbledore") && ch.contains("Voldemort"), "đã đổi tên riêng:\n{ch}");
    // Thực thể XML phải còn nguyên, không được giải mã thành `&` trần.
    assert!(ch.contains("&amp;"), "thực thể XML bị phá:\n{ch}");
    // Nội dung trong `<pre>` không phải văn xuôi.
    assert!(ch.contains("<pre>xử dụng</pre>"), "đã đụng vào <pre>:\n{ch}");
    // Thẻ inline phải còn nguyên si.
    assert!(ch.contains("khô<i>ng</i> bao giờ"), "đã xén thẻ inline:\n{ch}");
}

#[test]
fn tu_ghep_sua_duoc_ma_khong_can_mo_hinh() {
    let (_, ch) = chay("tughep");
    // `tình thuơng` → `tình thương`: từ điển có `tình thương` mà không có
    // `tình thường`. Bằng chứng dứt khoát nên sửa ngay, không cần mô hình.
    assert!(ch.contains("Tình thương của mẹ"), "chưa dùng bằng chứng từ ghép:\n{ch}");
}

#[test]
fn tieng_sai_khong_ai_quyet_duoc_thi_de_lai() {
    let (kq, ch) = chay("ngo");
    // `Ừ thuơng à` — không hàng xóm nào ghép thành từ có thật, và không có mô
    // hình. Không được đoán bừa: để nguyên, ghi vào danh sách chỗ ngờ.
    assert!(ch.contains("Ừ thuơng à"), "đã đoán bừa:\n{ch}");
    assert!(
        kq.chua_sua.iter().any(|c| c.goc == "thuơng"),
        "không ghi vào chỗ ngờ: {:?}",
        kq.chua_sua
    );
}

#[test]
fn file_ra_van_la_epub_hop_le() {
    let vao = cho_tam("cautruc-vao.epub");
    let ra = cho_tam("cautruc-ra.epub");
    dung_epub(&vao);
    chay_xu_ly(&vao, &ra);

    let byte = std::fs::read(&ra).unwrap();
    let mut kho = zip::ZipArchive::new(Cursor::new(byte)).unwrap();
    // `mimetype` phải là mục **đầu tiên** và **không nén**. Sai chỗ này thì
    // calibre vẫn mở được, còn máy đọc sách và epubcheck thì báo hỏng.
    let dau = kho.by_index(0).unwrap();
    assert_eq!(dau.name(), "mimetype");
    assert_eq!(dau.compression(), zip::CompressionMethod::Stored);
    drop(dau);
    // Đủ mục như bản gốc.
    assert_eq!(kho.len(), 4);
}

#[test]
fn file_khong_sua_thi_giu_nguyen_tung_byte() {
    let vao = cho_tam("nguyenven-vao.epub");
    let ra = cho_tam("nguyenven-ra.epub");
    dung_epub(&vao);
    chay_xu_ly(&vao, &ra);

    let a = Epub::nap(&vao).unwrap();
    let b = Epub::nap(&ra).unwrap();
    for ten in ["mimetype", "META-INF/container.xml", "OEBPS/content.opf"] {
        let x = a.muc.iter().find(|m| m.ten == ten).unwrap();
        let y = b.muc.iter().find(|m| m.ten == ten).unwrap();
        assert_eq!(x.noi_dung, y.noi_dung, "{ten} bị đổi mà không ai yêu cầu");
    }
}

#[test]
fn chay_lai_lan_hai_khong_doi_gi_nua() {
    // Bộ sửa phải **hội tụ**: sách đã sửa chạy lại thì ra đúng nó. Không hội tụ
    // nghĩa là hai tầng đang giằng nhau — tầng A đổi X thành Y rồi tầng B đổi Y
    // về X — và người dùng chạy hai lần ra hai kết quả khác nhau.
    //
    // Có đúng **một** ngoại lệ, và nó không phải giằng co: chỗ sửa vắt qua ranh
    // giới thẻ HTML thì lượt nào cũng tìm ra mà lượt nào cũng không vá được, vì
    // vá là phải xén thẻ. Chúng được đếm vào `vuong_the` và ghi rõ trong báo
    // cáo. Bài kiểm này ghim đúng ranh giới ấy: còn sót thì chỉ được sót loại
    // ấy, và số lượng phải khớp với số đã báo.
    let vao = cho_tam("hoitu-vao.epub");
    let ra1 = cho_tam("hoitu-ra1.epub");
    let ra2 = cho_tam("hoitu-ra2.epub");
    dung_epub(&vao);
    let kq1 = chay_xu_ly(&vao, &ra1);
    let kq2 = chay_xu_ly(&ra1, &ra2);

    assert!(kq1.vuong_the > 0, "EPUB thử nghiệm phải có ca vướng thẻ để bài kiểm có nghĩa");
    for s in &kq2.da_sua {
        assert_eq!(
            s.loai,
            chinhta::sua::Loai::KhoangTrang,
            "lượt hai sửa thứ không phải khoảng trắng vướng thẻ — bộ sửa giằng co: {s:?}"
        );
    }
    assert_eq!(kq2.vuong_the, kq1.vuong_the, "số chỗ vướng thẻ phải ổn định giữa hai lượt");

    // Và quan trọng nhất: hai file ra phải **giống hệt nhau**. Đây mới là điều
    // người dùng cảm nhận được — chạy lại không làm sách xấu đi thêm.
    let a = Epub::nap(&ra1).unwrap();
    let b = Epub::nap(&ra2).unwrap();
    for (x, y) in a.muc.iter().zip(b.muc.iter()) {
        assert_eq!(x.noi_dung, y.noi_dung, "{} đổi ở lượt hai", x.ten);
    }
}

#[test]
fn giu_nguyen_nhan_de_va_dem_dung_so_doan() {
    let (kq, _) = chay("thongke");
    assert_eq!(kq.nhan_de.as_deref(), Some("Sách thử nghiệm"));
    assert!(kq.so_doan >= 8, "đếm thiếu đoạn: {}", kq.so_doan);
}
