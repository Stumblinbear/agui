# if crate A depends on crate B, B must come before A in this list
crates=(
    agui_core
    agui
)

cd crates
for crate in "${crates[@]}"
do
  echo "Publishing ${crate}"
  (cd "$crate"; cargo publish --no-verify)
  sleep 20
done
