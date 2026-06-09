package generated

/*
#include <stdlib.h>
#include "b_wrapper.h"
*/
import "C"

import "errors"

type B struct {
    ptr *C.BHandle
    owned bool
    root *bool
}

func NewB() (*B, error) {
    ptr := C.cgowrap_B_new()
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedB(ptr), nil
}

func (b *B) Close() {
    if b == nil || b.ptr == nil {
        return
    }
    if !b.owned {
        return
    }
    if b.root != nil {
        *b.root = true
    }
    C.cgowrap_B_delete(b.ptr)
    b.ptr = nil
}

func newOwnedB(ptr *C.BHandle) *B {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &B{ptr: ptr, owned: true, root: root}
}

func newBorrowedB(ptr *C.BHandle, root *bool) *B {
    if ptr == nil {
        return nil
    }
    return &B{ptr: ptr, root: root}
}

func (b *B) Value() int32 {
    if b == nil || b.ptr == nil {
        return 0
    }
    if b.root != nil && *b.root {
        panic("B handle is closed")
    }
    return int32(C.cgowrap_B_Value(b.ptr))
}

func (b *B) SetValue(value int32) {
    if b == nil || b.ptr == nil {
        return
    }
    if b.root != nil && *b.root {
        panic("B handle is closed")
    }
    C.cgowrap_B_SetValue(b.ptr, C.int(value))
}
