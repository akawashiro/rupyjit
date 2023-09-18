import rupyjit
import dis

def test():
    print("Hello World!")

def add(a, b):
    return a + b

def sub(a, b):
    return a - b

def id(x):
    return x

def const():
    return 42

def use_if(x):
    if x:
        return 42
    else:
        return 24

dis.dis(use_if)

# print(rupyjit.version())
rupyjit.enable()

# test()
# add(4242, 2424)
# sub(4242, 2424)
use_if(True)
# id(42)
# const()
