# Maintainer: Yousif Haidar <asd22.info@gmail.com>
pkgname=rommer-git
pkgver=0.1.0.r0.g0000000
pkgrel=1
pkgdesc="A powerful tool to customize Android ROM ZIP files without building from source. Built with Rust"
arch=('any')
url="https://github.com/TheROMMER/core"
license=('GPL3')
depends=('gcc-libs')
makedepends=('cargo' 'git')
source=("git+$url.git")
sha256sums=('SKIP')

pkgver() {
	cd "$srcdir/core"
	printf "0.1.0.r%s.g%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

build() {
	cd "$srcdir/core"
	cargo build --release --locked
}

package() {
	cd "$srcdir/core"
	install -Dm755 "target/release/rommer" "$pkgdir/usr/bin/rommer"
	install -Dm644 "README.md" "$pkgdir/usr/share/doc/$pkgname/README.md"
	install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}