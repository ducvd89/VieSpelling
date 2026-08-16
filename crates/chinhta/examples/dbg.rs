use chinhta::tu_dien;
fn dem_thanh(s: &str) -> usize {
    s.chars().filter(|&c| chinhta::am_tiet::bo_thanh(c).1 != chinhta::am_tiet::NGANG).count()
}
fn main() {
    for c in ["ngồi ở", "ngồi xuống", "e rằng"] {
        println!("tu ghep {c:12} -> {}", tu_dien::co_tu_ghep(c));
    }
    println!("--- so dau thanh trong tung chuoi:");
    for t in ["ngồiở", "Huoàng", "phảii", "khuyếch", "tứước", "nòoài", "Ngooại", "hiệnh", "Phúlần", "erằng"] {
        println!("  {t:10} {} dau", dem_thanh(t));
    }
}
