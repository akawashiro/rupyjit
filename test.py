import rupyjit
import dis

def nested(n):
    return n

def test(n):
    return nested(n)

def fib(n):
    if n < 2:
        return 1
    return fib(n-1)
    # return fib(n-1) + fib(n-2)

# dis.dis(nested)
# dis.dis(test)

rupyjit.enable()

r = fib(2)
# r = nested(3)
# assert r == 3
