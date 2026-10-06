  ls | where size > 1kb
   ls | where name == "test"
  ls | where type == "file"
   $data | where size > 1kb
  $list | where name == "test"
   $table | where $it.value > 10
  $records | where status == "active" and age > 18
   $items | where {|row| $row.count > 0 }
  ls | where size > 1mb | where name =~ "\.rs$"
   $data | where { $in.field | is-not-empty }
