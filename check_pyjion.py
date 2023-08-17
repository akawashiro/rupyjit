#! /usr/bin/env python3
import pyjion
import pyjion.dis

def add(a, b):
    return a + b

pyjion.enable()
add(1, 2)
pyjion.disable()
pyjion.dis.dis_native(add)
