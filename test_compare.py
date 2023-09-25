import rupyjit

def compare(a, b):
    return a < b

rupyjit.enable()

r = compare(42, 24)
assert(r == False)
