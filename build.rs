// Sans script de build, cargo ne définit pas `OUT_DIR` — et les macros du SDK y écrivent les
// émissions dont `portaki build` assemble le manifeste. Ce fichier ne fait rien d'autre.
fn main() {
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=i18n/");
}
