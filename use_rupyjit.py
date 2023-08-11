import rupyjit

def test():
    print("Hello World!")

def add(a, b):
    return a + b

print(rupyjit.version())
rupyjit.enable()

# test()
add(4242, 1234)
