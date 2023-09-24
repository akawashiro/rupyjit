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

def fib(n):
    if n < 2:
        return n
    return fib(n-1) + fib(n-2)

def compare(a, b):
    return a < b

dis.dis(compare)

# print(rupyjit.version())
rupyjit.enable()

r = compare(42, 24)
print(r)
r = compare(24, 24)
print(r)
r = compare(24, 42)
print(r)
# r = fib(1)
# print(r)
# test()
# r = add(4242, 2424)
# print(r)
# sub(4242, 2424)
# r = use_if(True)
# print(r)
# r = use_if(False)
# print(r)
# id(42)
# r = const()
# print(r)
