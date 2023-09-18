import rupyjit

def test():
    print("Hello World!")

def add(a, b):
    return a + b

def id(x):
    return x

def const():
    return 42

print(rupyjit.version())
rupyjit.enable()

# test()
# add(4242, 1234)
id(42)
const()
