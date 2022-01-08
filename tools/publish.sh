# if crate A depends on crate B, B must come before A in this list
crates=(
    agui_core
    agui_macros
    agui_primitives
    agui_widgets
)

cd crates
for crate in "${crates[@]}"
do
  echo "Publishing ${crate}"
  (cd "$crate"; cargo publish --no-verify)
  sleep 20
done

cd ..
cargo publish