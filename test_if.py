import rupyjit

def use_if(x):
    if x:
        return 42
    else:
        return 24

rupyjit.enable()
r = use_if(True)
assert(r == 42)
r = use_if(False)
assert(r == 24)
