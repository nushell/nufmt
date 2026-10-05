export-env {
    $env.XXX_API_KEY = "..."

$env.PUB_HOSTED_URL = "..."
$env.FLUTTER_STORAGE_BASE_URL = "..."
}

export-env { $env.ONE_LINER = 1 }

export-env {}

module env_module {
export-env {
$env.NESTED = 1
}
}

export-env {} ignored
