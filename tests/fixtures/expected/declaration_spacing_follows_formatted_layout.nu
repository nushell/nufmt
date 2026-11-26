# A declaration that formats across lines is set apart from its group,
# whether it was written on one line or several.
const name = "fixtures"

# Every rule but one.
const rules = {
    policy: [drafts concurrency labels permissions pins]
}

const other = {
    policy: [pins]
}

const short = [alpha beta gamma]

const long = [
    alpha
    beta
    gamma
    delta
    epsilon
    zeta
    eta
    theta
    iota
    kappa
    lambda
    mu
    nu
    xi
    omicron
]

const after = 1

# A declaration that collapses onto one line joins its group again.
let first = 1
let empty = []
let single = [1]
let last = 2
