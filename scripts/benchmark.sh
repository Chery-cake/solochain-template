#!/bin/sh

die(){
	printf '[ERROR] %s' "$1"
	exit 1
}

# check for required programs
command -v dirname > /dev/null 2>&1 || die "'dirname' not found"
command -v cargo > /dev/null 2>&1 || die "'cargo' not found"

# go to project root directory
cd -- "$(dirname "${0}")" || exit 1
cd .. || exit 1

# check if the template file exists.
template_file='.maintain/frame-weight-template.hbs'
test -f "${template_file}" || die "frame weight template not found: ${template_file}"

# check if the binary exists, if not, build it
if test ! -x ./target/release/solochain-template-node; then
	echo 'solochain-template-node not found, building...'
	command='cargo build --release --features=runtime-benchmarks -p solochain-template-node'
	printf 'Building node with `%s`\n' "${command}"
	${command} || die 'Build failed'
fi

# run the benchmarks
./target/release/solochain-template-node benchmark pallet \
	--chain=dev \
	--pallet=pallet_template \
	--extrinsic='*' \
	--steps=50 \
	--repeat=200 \
	--wasm-execution=compiled \
	--output='pallets/template/src/weights.rs' \
	--template="${template_file}" || die 'Benchmark failed'

# done
echo "Done"
