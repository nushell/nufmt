let in_val = (
    $in_parsed
    | get -o tool_input
    | default ($in_parsed | get -o tool_response | default "")
)
