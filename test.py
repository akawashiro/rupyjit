import rupyjit
import dis

def fib(n):
    if n < 2:
        return n
    return fib(n-1) + fib(n-2)

dis.dis(fib)

rupyjit.enable()

r = fib(2)
print(r)
