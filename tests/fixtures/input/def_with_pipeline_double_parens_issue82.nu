def in-lang [ln] { ($in | items {|k v| {$k: ($v | get $ln | default ($v | get 'en')) } } | reduce -f {} {|it acc| $acc | merge $it }) }
