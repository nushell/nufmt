let state = if $is_new {
    init-state
    | upsert APP_DRIVER (get-field $item "APP_DRIVER")
    | upsert APP_STAGE (get-field $item "APP_STAGE")
    | upsert APP_TOOL (get-field $item "APP_TOOL")
}
