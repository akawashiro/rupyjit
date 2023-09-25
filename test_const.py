import rupyjit

def const():
    return 42

rupyjit.enable()
r = const()
assert(r == 42)
