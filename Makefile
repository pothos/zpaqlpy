zpaqlpydebug: target/debug/zpaqlpy
	cp target/debug/zpaqlpy zpaqlpydebug

zpaqlpy: target/release/zpaqlpy
	cp target/release/zpaqlpy zpaqlpy

target/debug/zpaqlpy:
	cargo build #  RUSTFLAGS="-Zincremental=target/INCREMENTAL -Zorbit"

target/release/zpaqlpy:
	cargo build --release

clean:
	rm target/debug/zpaqlpy target/release/zpaqlpy

check:
	# expects zpaqd to be in the top folder
	test/stress.py hcomp test/testcase test/splash.pypredict
	./zpaqlpydebug --run-hcomp test/testcase test/stress.py > test/splash.zpaqlpredict
	cmp test/splash.pypredict test/splash.zpaqlpredict
	./zpaqd c test/stress.cfg test/testar.zpaq test/min.pnm
	./zpaqd d test/testar.zpaq test/xy.pnm
	cmp test/min.pnm test/xy.pnm
	test/rle c test/testcase test/testcase.out
	test/rle_cm.py hcomp test/testcase.out test/xy.prydict  # not compared to r rle_cm.manual h min.out xy.manual yet
	./zpaqlpydebug --run-hcomp test/testcase.out test/rle_cm.py > test/xy.gen
	cmp test/xy.gen test/xy.prydict
	cd test && ../zpaqd c rle_cm.cfg testar.zpaq testcase && cd ..
	./zpaqd d test/testar.zpaq test/testcase.xy
	cmp test/testcase test/testcase.xy
	test/rle c test/min.pnm test/min.out
	./zpaqd r test/rle_cm.manual.cfg p test/min.out test/xy.pnm
	cmp test/min.pnm test/xy.pnm
	./zpaqlpydebug test/lz1.py
	cd test && ../zpaqd c lz1.cfg testar.zpaq rafale.pnm peppers.pnm monarch.pnm kodim23.pnm && cd ..
	test/arrays.py pcomp test/lz1.py
	./zpaqlpydebug test/arrays.py
	./zpaqd r test/arrays.cfg p test/lz1.py
	./zpaqd r test/arrays.cfg h test/lz1.py > /dev/null
	test/simple_rle test/testcase test/testcase.simple
	test/rle_model.py hcomp test/testcase.simple test/testcase.predictpy
	./zpaqlpydebug --run-hcomp test/testcase.simple test/rle_model.py > test/testcase.predictz
	cmp test/testcase.predictz test/testcase.predictpy
	echo | test/rle_model.py --compare test/testcase pcomp test/testcase.simple test/testcase.origpy
	./zpaqd r test/rle_model.cfg p test/testcase.simple test/testcase.origz
	cmp test/testcase.origpy test/testcase.origz
	./zpaqlpydebug test/pnm.py
	test/subtract_green test/min.pnm test/min.sub.pnm
	echo | test/pnm.py --compare test/min.pnm pcomp test/min.sub.pnm /dev/null

benchmark:
	RUST_BACKTRACE=1 ./zpaqlpydebug test/pnm.py
	cd test && ../zpaqd c pnm.cfg testar.zpaq rafale.pnm peppers.pnm monarch.pnm kodim23.pnm && cd .. && ls -l test/testar.zpaq

brotlitest:
	RUST_BACKTRACE=1 ./zpaqlpydebug test/brotli.py
	touch test/dict_is_present.tmp
	rm test/dict_is_present.tmp
	cd test && ../zpaqd c brotli.cfg testar.zpaq testcase rafale.pnm peppers.pnm monarch.pnm kodim23.pnm && cd .. && ls -l test/testar.zpaq
	# run on input: ../zpaqd r brotli.cfg p pre.dict.br pre.out
	# debug: ../zpaqd t brotli.cfg p `od -A none -t x1 -v pre.dict.br | tr -d '\n'`
	# use: ./zpaq x test/testar.zpaq -to various
	# rm dict_is_present.tmp ; ../zpaqd c brotli.cfg empty.zpaq /dev/null ; ls -l empty.zpaq
	# rm dict_is_present.tmp ; ../zpaqd c brotli.cfg testar.zpaq rafale.pnm peppers.pnm monarch.pnm kodim23.pnm ; ls -l testar.zpaq

otherbenchmarks:
	cd test && ../zpaqd c bmp_j4.cfg testar.zpaq rafale.bmp peppers.bmp monarch.bmp kodim23.bmp && cd .. && ls -l test/testar.zpaq
	cd test && ../zpaqd c pnm.cfg empty.zpaq /dev/null && cd .. && ls -l test/empty.zpaq
	cd test && ../zpaqd c bmp_j4.cfg empty.zpaq /dev/null && cd .. && ls -l test/empty.zpaq
	cd test && time ../zpaqd c lz1.orig.cfg testar.zpaq rafale.pnm peppers.pnm monarch.pnm kodim23.pnm && cd .. && ls -l test/testar.zpaq
	cd test && time ../zpaqd c lz1.cfg testar.zpaq rafale.pnm peppers.pnm monarch.pnm kodim23.pnm && cd .. && ls -l test/testar.zpaq
	cd test && ../zpaqd c lz1.cfg empty.zpaq /dev/null && cd .. && ls -l test/empty.zpaq
	cd test && ../zpaqd c lz1.orig.cfg empty.zpaq /dev/null && cd .. && ls -l test/empty.zpaq
	du -cb test/*webp
	du -cb test/*flif
	du -cb test/*png
