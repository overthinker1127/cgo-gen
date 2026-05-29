package generated

/*
#include <stdlib.h>
#include "a_wrapper.h"
*/
import "C"

import "errors"

type A struct {
    ptr *C.AHandle
    owned bool
    root *bool
}

func NewA() (*A, error) {
    ptr := C.cgowrap_A_new()
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedA(ptr), nil
}

func (a *A) Close() {
    if a == nil || a.ptr == nil {
        return
    }
    if !a.owned {
        return
    }
    if a.root != nil {
        *a.root = true
    }
    C.cgowrap_A_delete(a.ptr)
    a.ptr = nil
}

func newOwnedA(ptr *C.AHandle) *A {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &A{ptr: ptr, owned: true, root: root}
}

func newBorrowedA(ptr *C.AHandle, root *bool) *A {
    if ptr == nil {
        return nil
    }
    return &A{ptr: ptr, root: root}
}

func (a *A) Child() *B {
    if a == nil || a.ptr == nil {
        return nil
    }
    if a.root != nil && *a.root {
        panic("A handle is closed")
    }
    raw := C.cgowrap_A_Child(a.ptr)
    if raw == nil {
        return nil
    }
    return newBorrowedB(raw, a.root)
}

func (a *A) ChildValue() int32 {
    if a == nil || a.ptr == nil {
        return 0
    }
    if a.root != nil && *a.root {
        panic("A handle is closed")
    }
    return int32(C.cgowrap_A_ChildValue(a.ptr))
}

func (a *A) SetChildValue(value int32) {
    if a == nil || a.ptr == nil {
        return
    }
    if a.root != nil && *a.root {
        panic("A handle is closed")
    }
    C.cgowrap_A_SetChildValue(a.ptr, C.int(value))
}
