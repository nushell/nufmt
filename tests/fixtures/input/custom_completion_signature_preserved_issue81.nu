def f1 [] { }
def f2 [foo: int@f1 = 1] { }
def f3 [x: string@[a b c]] { }
def "complete foo" [] { }
def test [x: int@"complete foo"] { }
