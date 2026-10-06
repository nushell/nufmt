error make  {msg:  "simple error"}
error make   {msg:"simple error"}
error make  {
    msg:  "error message"
    label:  {text:  "label text"}
}
error make   {
    msg:   "error"
    label:   {
        text:  "here"
        span:   $span
    }
}
error make  {msg:   "test error"}
error make   {
    msg:  "detailed error"
    label:   {text:  "error occurred here"}
}
error make  {
    msg:   $"interpolated ($value) error"
}
if $invalid  {  error make  {msg:  "validation failed"}  }
