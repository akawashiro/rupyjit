import rupyjit
print(rupyjit.version())
rupyjit.enable()

def test():
    print("Hello World!")

def add(a, b):
    return a + b

test()
add(1, 2)
