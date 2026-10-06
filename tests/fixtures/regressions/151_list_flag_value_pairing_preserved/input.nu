let base_args: list<string> = [
  "--no-pager"
  "--output" "json"
  "--identifier" "inceptool"
  "--lines" ($lines | into string)
  "--priority" ($max_pri | into string)
]
