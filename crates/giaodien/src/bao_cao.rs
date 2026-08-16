//! Viết báo cáo.
//!
//! Ứng dụng tự sửa rồi mới báo, nên **báo cáo là thứ duy nhất người dùng có để
//! kiểm lại**. Vì thế nó phải trả lời được ba câu, theo đúng thứ tự này:
//!
//! 1. Đã đổi những gì — nguyên văn trước và sau, không tóm tắt.
//! 2. Vì sao đổi — luật nào bắt, hay mô hình chấm hơn bao nhiêu.
//! 3. Chỗ nào **ngờ mà không đổi** — phần này quan trọng ngang phần đã đổi, vì
//!    nó là danh sách việc còn lại cho người biên tập.
//!
//! Gom các phép sửa giống hệt nhau lại thành một dòng kèm số lần. Một cuốn sách
//! có thể có 400 chỗ `xử dụng`; liệt kê 400 dòng thì phần còn lại của báo cáo
//! chìm nghỉm.

use crate::xu_ly::KetQuaSach;
use chinhta::sua::DoChac;
use std::collections::BTreeMap;
use std::fmt::Write;

/// Gom phép sửa trùng nhau: (loại, gốc, mới, lý do) → số lần.
fn gom(kq: &KetQuaSach) -> BTreeMap<&'static str, Vec<(String, String, String, usize)>> {
    let mut theo_loai: BTreeMap<&'static str, BTreeMap<(String, String), (String, usize)>> =
        BTreeMap::new();
    for s in &kq.da_sua {
        let muc = theo_loai.entry(s.loai.ten()).or_default();
        let e = muc
            .entry((s.goc.clone(), s.thay_bang.clone()))
            .or_insert((s.ly_do.clone(), 0));
        e.1 += 1;
    }
    theo_loai
        .into_iter()
        .map(|(loai, m)| {
            let mut v: Vec<_> = m
                .into_iter()
                .map(|((goc, moi), (ly_do, n))| (goc, moi, ly_do, n))
                .collect();
            v.sort_by_key(|(_, _, _, n)| std::cmp::Reverse(*n));
            (loai, v)
        })
        .collect()
}

fn thoat(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Hiện khoảng trắng và ký tự vô hình ra dạng nhìn thấy được.
///
/// Không có bước này thì dòng báo cáo về khoảng trắng đọc thành `đổi " " thành
/// " "` — đúng nhưng vô dụng.
fn hien(s: &str) -> String {
    if s.is_empty() {
        return "∅".into();
    }
    let mut ra = String::new();
    for c in s.chars() {
        match c {
            ' ' => ra.push('␣'),
            '\t' => ra.push('⇥'),
            '\n' => ra.push('⏎'),
            c if c.is_control() || (c as u32) == 0xAD || (0x200B..=0x200F).contains(&(c as u32)) => {
                let _ = write!(ra, "⟨U+{:04X}⟩", c as u32);
            }
            c => ra.push(c),
        }
    }
    ra
}

/// Hiện một cặp trước/sau, có thêm mã ký tự khi hai vế **nhìn giống hệt nhau**.
///
/// Lỗi Unicode là lỗi vô hình theo đúng nghĩa đen: `ế` gõ liền và `ế` gõ rời
/// (e + dấu mũ + dấu sắc) hiện ra y hệt nhau. Dòng báo cáo "đổi ế thành ế"
/// không nói được gì; phải chỉ ra chỗ khác nhau nằm ở mã ký tự.
fn cap(goc: &str, moi: &str) -> (String, String) {
    use unicode_normalization::UnicodeNormalization;
    let (a, b) = (hien(goc), hien(moi));
    // So sau khi dựng lại NFC, **không** so chuỗi byte: `Pháp` gõ rời và `Pháp`
    // gõ liền là hai chuỗi byte khác nhau nhưng hiện ra một hình. So byte thì
    // hàm này kết luận "hai vế khác nhau rồi" và bỏ qua đúng ca nó sinh ra để
    // xử lý — cả bảng Unicode trong báo cáo đọc thành "Pháp → Pháp".
    let nhu_nhau = a.nfc().eq(b.nfc());
    if !nhu_nhau {
        return (a, b);
    }
    let ma = |s: &str| {
        s.chars().map(|c| format!("U+{:04X}", c as u32)).collect::<Vec<_>>().join(" ")
    };
    (format!("{a} ⟨{}⟩", ma(goc)), format!("{b} ⟨{}⟩", ma(moi)))
}

pub fn html(kq: &KetQuaSach) -> String {
    let mut h = String::with_capacity(64 * 1024);
    let nhan_de = kq.nhan_de.clone().unwrap_or_else(|| "(không rõ nhan đề)".into());
    let dang_ke = crate::xu_ly::so_loi_dang_ke(kq);

    h.push_str(
        r#"<!doctype html><html lang="vi"><head><meta charset="utf-8">
<title>Báo cáo sửa chính tả</title><style>
:root{color-scheme:light dark;--nen:#fff;--chu:#1a1a1a;--mo:#666;--vien:#e0e0e0;--do:#c0392b;--xanh:#1e7e34;--vang:#8a6d1f;--nennhe:#fafafa}
@media(prefers-color-scheme:dark){:root{--nen:#16181c;--chu:#e6e6e6;--mo:#9aa0a6;--vien:#2c3038;--do:#ff8a80;--xanh:#7ee08a;--vang:#e8c86a;--nennhe:#1c1f24}}
*{box-sizing:border-box}
body{background:var(--nen);color:var(--chu);font:16px/1.6 -apple-system,"Segoe UI",system-ui,sans-serif;margin:0;padding:2rem 1.25rem;max-width:60rem;margin-inline:auto}
h1{font-size:1.6rem;margin:0 0 .25rem}
h2{font-size:1.15rem;margin:2.5rem 0 .75rem;padding-bottom:.35rem;border-bottom:1px solid var(--vien)}
.phu{color:var(--mo);margin:0 0 2rem}
.the{display:flex;flex-wrap:wrap;gap:.75rem;margin:1.5rem 0}
.o{border:1px solid var(--vien);border-radius:.5rem;padding:.6rem .9rem;background:var(--nennhe);min-width:8rem}
.o b{display:block;font-size:1.5rem;line-height:1.2}
.o span{color:var(--mo);font-size:.85rem}
table{border-collapse:collapse;width:100%;font-size:.94rem}
th,td{text-align:left;padding:.45rem .6rem;border-bottom:1px solid var(--vien);vertical-align:top}
th{color:var(--mo);font-weight:600;font-size:.85rem;text-transform:uppercase;letter-spacing:.03em}
td.n{text-align:right;color:var(--mo);white-space:nowrap}
code{font-family:ui-monospace,"Cascadia Code",Consolas,monospace;font-size:.92em;background:var(--nennhe);padding:.1em .35em;border-radius:.25em}
.cu{color:var(--do)}.moi{color:var(--xanh)}
.mui{color:var(--mo);padding:0 .4em}
.ghi{color:var(--mo);font-size:.88rem}
.canh{border-left:3px solid var(--vang);background:var(--nennhe);padding:.75rem 1rem;border-radius:0 .4rem .4rem 0;margin:1rem 0}
.cuon{overflow-x:auto}
</style></head><body>"#,
    );

    let _ = write!(h, "<h1>{}</h1>", thoat(&nhan_de));
    let _ = write!(
        h,
        r#"<p class="phu">Báo cáo sửa chính tả — {} file nội dung, {} đoạn, {} chữ.</p>"#,
        kq.so_file,
        kq.so_doan,
        kq.so_chu
    );

    // Bảng số liệu.
    h.push_str(r#"<div class="the">"#);
    let _ = write!(h, r#"<div class="o"><b>{}</b><span>phép sửa</span></div>"#, kq.da_sua.len());
    let _ = write!(h, r#"<div class="o"><b>{dang_ke}</b><span>lỗi chữ nghĩa</span></div>"#);
    let _ = write!(
        h,
        r#"<div class="o"><b>{}</b><span>chỗ ngờ, chưa sửa</span></div>"#,
        kq.chua_sua.len()
    );
    let _ = write!(
        h,
        r#"<div class="o"><b>{}</b><span>kiểu đặt dấu</span></div>"#,
        if kq.kieu_dau == chinhta::dau_thanh::Kieu::Cu { "cũ" } else { "mới" }
    );
    h.push_str("</div>");

    if !kq.co_mo_hinh {
        h.push_str(
            r#"<div class="canh"><b>Chạy không có mô hình ngôn ngữ.</b> Những chỗ cần hiểu
            câu mới phân được đều nằm nguyên ở mục <i>chỗ ngờ</i> bên dưới chứ không được
            sửa. Chọn một file mô hình GGUF trong phần Cài đặt để bật tầng này.</div>"#,
        );
    }
    if kq.vuong_the > 0 {
        let _ = write!(
            h,
            r#"<div class="canh"><b>{} chỗ tìm ra nhưng không sửa được</b> vì chữ bị thẻ HTML
            cắt ngang (kiểu <code>khô&lt;i&gt;ng&lt;/i&gt;</code>). Sửa được thì phải xoá thẻ,
            mà đó là đổi cách trình bày chứ không phải sửa chính tả.</div>"#,
            kq.vuong_the
        );
    }
    if kq.dem_kieu.ty_le_thieu_so() > 0.2 {
        let _ = write!(
            h,
            r#"<div class="canh"><b>Sách vốn không nhất quán kiểu đặt dấu thanh</b> — {} chỗ
            kiểu cũ (<code>hòa</code>) và {} chỗ kiểu mới (<code>hoà</code>). Cả hai đều đúng
            chính tả, nên đây là lựa chọn trình bày. Đã kéo về kiểu chiếm đa số.</div>"#,
            kq.dem_kieu.cu, kq.dem_kieu.moi
        );
    }

    // Đã sửa, gom theo loại.
    let nhom = gom(kq);
    if nhom.is_empty() {
        h.push_str("<h2>Đã sửa</h2><p class=\"ghi\">Không có gì để sửa.</p>");
    }
    for (loai, muc) in &nhom {
        let tong: usize = muc.iter().map(|(_, _, _, n)| n).sum();
        let _ = write!(h, "<h2>{} <span class=\"ghi\">— {tong} chỗ</span></h2>", thoat(loai));
        h.push_str(r#"<div class="cuon"><table><tr><th>Sửa</th><th>Vì sao</th><th>Số lần</th></tr>"#);
        for (goc, moi, ly_do, n) in muc.iter().take(200) {
            let (a, b) = cap(goc, moi);
            let _ = write!(
                h,
                r#"<tr><td><code class="cu">{}</code><span class="mui">→</span><code class="moi">{}</code></td><td class="ghi">{}</td><td class="n">{}</td></tr>"#,
                thoat(&a),
                thoat(&b),
                thoat(ly_do),
                n
            );
        }
        if muc.len() > 200 {
            let _ = write!(
                h,
                r#"<tr><td colspan="3" class="ghi">… và {} kiểu sửa khác cùng loại</td></tr>"#,
                muc.len() - 200
            );
        }
        h.push_str("</table></div>");
    }

    // Chưa sửa.
    h.push_str("<h2>Chỗ ngờ, chưa sửa</h2>");
    if kq.chua_sua.is_empty() {
        h.push_str("<p class=\"ghi\">Không còn chỗ nào để ngỏ.</p>");
    } else {
        h.push_str(
            r#"<p class="ghi">Những chỗ bộ dò thấy đáng ngờ nhưng không đủ căn cứ để tự đổi.
            Đây là danh sách việc còn lại — kiểm bằng mắt rồi sửa tay.</p>"#,
        );
        let mut gom_ngo: BTreeMap<(String, String), usize> = BTreeMap::new();
        for c in &kq.chua_sua {
            let uv = c.ung_vien.join(", ");
            *gom_ngo.entry((c.goc.clone(), uv)).or_insert(0) += 1;
        }
        let mut v: Vec<_> = gom_ngo.into_iter().collect();
        v.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        h.push_str(
            r#"<div class="cuon"><table><tr><th>Chữ trong sách</th><th>Có thể là</th><th>Số lần</th></tr>"#,
        );
        for ((goc, uv), n) in v.iter().take(300) {
            let _ = write!(
                h,
                r#"<tr><td><code class="cu">{}</code></td><td class="ghi">{}</td><td class="n">{}</td></tr>"#,
                thoat(goc),
                thoat(uv),
                n
            );
        }
        h.push_str("</table></div>");
    }

    h.push_str("</body></html>");
    h
}

/// Bản chữ thuần, cho người muốn đọc trong terminal hoặc chép vào ghi chú.
pub fn chu(kq: &KetQuaSach) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "{}\n{} file, {} đoạn, {} chữ",
        kq.nhan_de.clone().unwrap_or_else(|| "(không rõ nhan đề)".into()),
        kq.so_file,
        kq.so_doan,
        kq.so_chu
    );
    let _ = writeln!(
        s,
        "{} phép sửa ({} lỗi chữ nghĩa), {} chỗ ngờ chưa sửa",
        kq.da_sua.len(),
        crate::xu_ly::so_loi_dang_ke(kq),
        kq.chua_sua.len()
    );
    if kq.vuong_the > 0 {
        let _ = writeln!(s, "{} chỗ vướng thẻ HTML nên không vá được", kq.vuong_the);
    }
    s.push('\n');
    for (loai, muc) in gom(kq) {
        let tong: usize = muc.iter().map(|(_, _, _, n)| n).sum();
        let _ = writeln!(s, "── {loai} ({tong} chỗ)");
        for (goc, moi, _, n) in muc.iter().take(40) {
            let (a, b) = cap(goc, moi);
            let _ = writeln!(s, "   {a} → {b}   ×{n}");
        }
        if muc.len() > 40 {
            let _ = writeln!(s, "   … và {} kiểu khác", muc.len() - 40);
        }
        s.push('\n');
    }
    if !kq.chua_sua.is_empty() {
        let _ = writeln!(s, "── Chỗ ngờ, chưa sửa ({})", kq.chua_sua.len());
        for c in kq.chua_sua.iter().take(40) {
            let _ = writeln!(s, "   {}  →?  {}", c.goc, c.ung_vien.join(", "));
        }
    }
    s
}

/// Đúng một dòng, cho thanh trạng thái.
pub fn mot_dong(kq: &KetQuaSach) -> String {
    let ngo = if kq.chua_sua.is_empty() {
        String::new()
    } else {
        format!(", {} chỗ để ngỏ", kq.chua_sua.len())
    };
    format!(
        "Đã sửa {} chỗ ({} lỗi chữ nghĩa){} trên {} đoạn.",
        kq.da_sua.len(),
        crate::xu_ly::so_loi_dang_ke(kq),
        ngo,
        kq.so_doan
    )
}

/// Số phép sửa có độ chắc thấp — hiện riêng để người dùng biết nên soi kỹ tới đâu.
pub fn so_ngo_vuc(kq: &KetQuaSach) -> usize {
    kq.da_sua.iter().filter(|s| s.do_chac == DoChac::NgoVuc).count()
}
