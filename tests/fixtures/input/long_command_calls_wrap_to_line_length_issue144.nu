if $outcome.skipped {
    echo $acc.passed ($acc.failed + 1) $acc.skipped ($acc.failures | append $outcome.failure)
}
