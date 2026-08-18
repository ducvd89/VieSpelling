//! So hai lối hỏi mô hình trên những ca bẫy **lấy từ chính sách thật**.
//!
//! ```bash
//! cargo run --release -p mohinh --example so_loi_cham -- mo-hinh.gguf sach.epub
//! ```
//!
//! Vì sao không cắm sẵn mấy đoạn văn vào đây cho gọn: đoạn văn trong sách dịch
//! là của người khác. Công cụ này chỉ giữ **danh sách chữ** cùng điều đúng phải
//! xảy ra với từng chữ — đó là sự thật về lỗi gõ, không phải văn của ai — rồi đi
//! tìm chúng trong cuốn sách người dùng chỉ vào, lấy luôn hai đoạn kề bên làm
//! ngữ cảnh. Đổi sách khác cũng chạy được, chỉ là tìm không thấy ca nào.
//!
//! Đây là chỗ **duy nhất** đo được cái mà con số tổng không nói ra. Hai lối chấm
//! bắt gần bằng nhau về số lượng (126 so với 124 lỗi trên tập 4 Harry Potter),
//! nhưng chúng không bắt cùng một tập hợp: 8 chỗ chúng quyết khác nhau, và mỗi
//! bên sai một kiểu riêng. Đếm tổng thì hai lối trông như nhau; nhìn vào 8 chỗ ấy
//! mới thấy chúng khác hẳn.

use chinhta::dau_thanh::Kieu;
use chinhta::soat::{BoSoat, KieuCham, NguCanh, TuyChon};
use std::collections::HashMap;

/// Điều **đúng** phải xảy ra với một chữ.
enum Dung {
    /// Không được đụng tới. Sửa là làm hỏng.
    Giu,
    /// Phải sửa, và phải ra đúng chữ này.
    Sua(&'static str),
}

struct Ca {
    chu: &'static str,
    dung: Dung,
    vi_sao: &'static str,
}

/// Ca bẫy đếm được trên tập 4 Harry Potter.
///
/// Chọn theo đúng một tiêu chí: **những chỗ hai lối chấm quyết khác nhau**, cộng
/// mấy ca dễ để bắt lỗi hồi quy. Nửa danh sách là ca không được sửa, và đó không
/// phải cho cân đối cho đẹp — sửa nhầm đắt hơn bỏ sót nhiều lần, nên phần nào
/// cũng phải đo là phần ấy.
const CA: &[Ca] = &[
    Ca {
        chu: "lôga",
        dung: Dung::Giu,
        vi_sao: "`thước lôga` là cái thước tính, không phải lỗi gõ",
    },
    Ca {
        chu: "zăc",
        dung: Dung::Giu,
        vi_sao: "`zic-zăc` phiên âm từ nước ngoài",
    },
    Ca {
        chu: "quọ",
        dung: Dung::Giu,
        vi_sao: "`quạu quọ` là từ có thật, chỉ là từ điển thiếu",
    },
    Ca {
        chu: "zợi",
        dung: Dung::Giu,
        vi_sao: "giọng Pháp của Fleur — người dịch cố ý viết chệch (`dững`, `cũa`, `chẵng`)",
    },
    Ca {
        chu: "ghứ",
        dung: Dung::Giu,
        vi_sao: "cũng giọng Fleur; sửa thành `thứ` là đúng chữ mà mất giọng nhân vật",
    },
    Ca {
        chu: "shứ",
        dung: Dung::Giu,
        vi_sao: "cũng giọng Fleur",
    },
    Ca { chu: "đựoc", dung: Dung::Sua("được"), vi_sao: "đảo hai chữ, lặp 4 lần trong sách" },
    Ca { chu: "đuọc", dung: Dung::Sua("được"), vi_sao: "lạc dấu phụ" },
    Ca { chu: "thuớc", dung: Dung::Sua("thước"), vi_sao: "lạc dấu phụ" },
    Ca { chu: "vửa", dung: Dung::Sua("vừa"), vi_sao: "sai dấu thanh" },
    Ca { chu: "bôj", dung: Dung::Sua("bộ"), vi_sao: "gõ nhầm phím kề" },
    Ca { chu: "nhữn", dung: Dung::Sua("những"), vi_sao: "thiếu một chữ cái" },
];

fn main() -> anyhow::Result<()> {
    let mut d = std::env::args().skip(1);
    let (Some(dm), Some(ds)) = (d.next(), d.next()) else {
        anyhow::bail!("cần: <mô-hình.gguf> <sách.epub>");
    };
    eprintln!("Nạp mô hình…");
    let mh = mohinh::MoHinh::nap(std::path::Path::new(&dm))?;
    eprintln!("{}", mh.mo_ta());

    kiem_dem_tien_to(&mh)?;

    let doan = doc_doan(std::path::Path::new(&ds))?;
    eprintln!("{} đoạn trong sách.\n", doan.len());

    // Tên riêng đếm từ chính cuốn sách, y như đường chạy thật — thiếu bước này
    // thì mọi tên phiên âm thành ra ca bẫy giả.
    let mut dem = HashMap::new();
    for d in &doan {
        chinhta::soat::gom_ten_rieng(d, &mut dem);
    }
    let ten_rieng = chinhta::soat::chot_ten_rieng(dem);

    // Tìm ca trong sách một lần, dùng cho mọi lượt đo.
    let mut cho: Vec<(&Ca, usize)> = Vec::new();
    for ca in CA {
        match doan.iter().position(|d| co_chu(d, ca.chu)) {
            Some(i) => cho.push((ca, i)),
            None => println!("({} không có trong sách này)", ca.chu),
        }
    }
    let co = cho.len();

    let chay = |kieu: KieuCham, nguong: f32| -> Vec<(String, bool)> {
        let bo = BoSoat::moi(
            TuyChon { kieu_cham: kieu, nguong_mo_hinh: nguong, ..TuyChon::default() },
            Kieu::Cu,
        )
        .voi_ten_rieng(ten_rieng.clone());
        cho.iter()
            .map(|(ca, i)| {
                let nc = NguCanh {
                    truoc: i.checked_sub(1).map(|k| doan[k].as_str()).unwrap_or(""),
                    sau: doan.get(i + 1).map(|x| x.as_str()).unwrap_or(""),
                };
                let mut kq = bo.soat(&doan[*i]);
                bo.quyet_bang_mo_hinh(&mut kq, &mh, &nc, &mut |_, _| {});
                let ra = kq.da_sua.iter().find(|s| s.goc == ca.chu).map(|s| s.thay_bang.clone());
                let dat = match (&ca.dung, &ra) {
                    (Dung::Giu, None) => true,
                    (Dung::Sua(x), Some(y)) => x == y,
                    _ => false,
                };
                (ra.unwrap_or_else(|| "(giữ nguyên)".into()), dat)
            })
            .collect()
    };

    // **Quét ngưỡng cho cả hai lối.** Không quét thì phép so vô nghĩa: lối điền
    // chỗ trống chia điểm cho một khoảng dài hơn nên biên độ hơn-kém của nó nhỏ
    // hơn, tức là ở cùng con số ngưỡng nó đang bị **chặn chặt hơn**. Đo ở một
    // ngưỡng duy nhất thì không phân được "chấm khéo hơn" với "bị chặn chặt hơn".
    println!("\nSố ca đúng trên {co}, theo ngưỡng:");
    println!("{:>9}  {:>8}  {:>10}", "ngưỡng", "cả câu", "chỗ trống");
    let mut tot = (0usize, 0.0f32);
    for n in [0.01f32, 0.03, 0.06, 0.10, 0.15, 0.25] {
        let a = chay(KieuCham::CaCau, n).iter().filter(|x| x.1).count();
        let b = chay(KieuCham::ChoTrong, n).iter().filter(|x| x.1).count();
        if b > tot.0 {
            tot = (b, n);
        }
        println!("{n:>9.2}  {a:>8}  {b:>10}");
    }

    // Bảng chi tiết ở ngưỡng mặc định, để đọc **kiểu sai** chứ không chỉ số sai.
    let a = chay(KieuCham::CaCau, TuyChon::default().nguong_mo_hinh);
    let b = chay(KieuCham::ChoTrong, tot.1);
    println!("\nChi tiết — cả câu ở ngưỡng {:.2}, chỗ trống ở ngưỡng {:.2}:", 0.03, tot.1);
    println!("{}", "─".repeat(76));
    for (k, (ca, _)) in cho.iter().enumerate() {
        println!(
            "{:<8} {:<26} {:<26}   {}",
            ca.chu,
            format!("{} {}", if a[k].1 { "✓" } else { "✗" }, a[k].0),
            format!("{} {}", if b[k].1 { "✓" } else { "✗" }, b[k].0),
            ca.vi_sao
        );
    }
    println!("{}", "─".repeat(76));
    println!("Mô hình chạy {} lượt chấm.", mh.so_luot.get());
    Ok(())
}

/// Bộ đệm tiền tố phải cho ra **đúng** con số như lúc nạp lại từ đầu.
///
/// Kiểm trước mọi thứ khác, vì đây là chỗ hỏng lặng lẽ nhất của lối điền chỗ
/// trống: đệm sai thì ứng viên được chấm trong một ngữ cảnh khác ngữ cảnh ta
/// tưởng, mà chương trình vẫn chạy trơn và vẫn in ra những con số trông hợp lý.
fn kiem_dem_tien_to(mh: &mohinh::MoHinh) -> anyhow::Result<()> {
    use chinhta::soat::ChamDiem;
    let truoc = "Trời đã tối hẳn. Hắn ngồi im rất lâu, rồi đứng lên và nói: ";
    let sau = " đi thôi, muộn rồi.";
    // Lần đầu: đệm nguội, phải nạp cả tiền tố.
    let a = mh.cham_cho_trong(truoc, "thôi", sau);
    // Chấm một câu khác — `cham` xoá sạch bộ đệm KV, nên lượt sau lại nguội.
    let _ = mh.cham("Một câu chẳng liên quan gì.");
    let b = mh.cham_cho_trong(truoc, "thôi", sau);
    // Lần thứ ba: đệm **nóng**, tiền tố giữ lại được.
    let c = mh.cham_cho_trong(truoc, "thôi", sau);
    if (a - b).abs() > 1e-4 || (a - c).abs() > 1e-4 {
        anyhow::bail!("bộ đệm tiền tố cho ra điểm khác nhau: nguội {a:.6}, nguội lại {b:.6}, nóng {c:.6}");
    }
    eprintln!("Bộ đệm tiền tố: khớp ({a:.6}).");
    Ok(())
}

/// Chữ có đứng riêng thành một tiếng trong đoạn không — `co` không được khớp vào
/// giữa `con`.
fn co_chu(doan: &str, chu: &str) -> bool {
    chinhta::tach_tu::cat(doan).iter().any(|t| t.chu == chu)
}

/// Mọi đoạn văn của sách, đã dựng lại NFC.
fn doc_doan(p: &std::path::Path) -> anyhow::Result<Vec<String>> {
    let epub = sach::Epub::nap(p)?;
    let mut ra = Vec::new();
    for i in epub.chi_so_van_ban() {
        let Some(nd) = sach::doc_chuoi(&epub.muc[i].noi_dung) else { continue };
        for d in sach::quet::quet(&nd) {
            ra.push(chinhta::chuan_hoa::dung_lai_nfc(&d.chu).unwrap_or(d.chu));
        }
    }
    Ok(ra)
}
