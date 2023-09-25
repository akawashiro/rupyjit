import rupyjit

def add(a, b):
    return a + b

def sub(a, b):
    return a - b

rupyjit.enable()

r = add(4242, 2424)
assert(r == 6666)

r = sub(4242, 2424)
assert(r == 1818)
