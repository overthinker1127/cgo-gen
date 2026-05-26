package generated

/*
#include <stdlib.h>
#include "vector_2_wrapper.h"
*/
import "C"

import "errors"

func OperMinus(lhs *Vector2, rhs *Vector2) *Vector2 {
    var cArg0 *C.Vector2Handle
    if lhs == nil || lhs.ptr == nil {
        panic("Vector2 handle is required but nil")
    }
    if lhs.root != nil && *lhs.root {
        panic("Vector2 handle is closed")
    }
    cArg0 = lhs.ptr
    var cArg1 *C.Vector2Handle
    if rhs == nil || rhs.ptr == nil {
        panic("Vector2 handle is required but nil")
    }
    if rhs.root != nil && *rhs.root {
        panic("Vector2 handle is closed")
    }
    cArg1 = rhs.ptr
    raw := C.cgowrap_OperMinus(cArg0, cArg1)
    if raw == nil {
        return nil
    }
    return newOwnedVector2(raw)
}

type Vector2 struct {
    ptr *C.Vector2Handle
    owned bool
    root *bool
}

func NewVector2() (*Vector2, error) {
    ptr := C.cgowrap_Vector2_new__void()
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedVector2(ptr), nil
}

func NewVector2WithXY(x int32, y int32) (*Vector2, error) {
    ptr := C.cgowrap_Vector2_new__int_int(C.int(x), C.int(y))
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedVector2(ptr), nil
}

func (v *Vector2) Close() {
    if v == nil || v.ptr == nil {
        return
    }
    if !v.owned {
        return
    }
    if v.root != nil {
        *v.root = true
    }
    C.cgowrap_Vector2_delete(v.ptr)
    v.ptr = nil
}

func newOwnedVector2(ptr *C.Vector2Handle) *Vector2 {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &Vector2{ptr: ptr, owned: true, root: root}
}

func newBorrowedVector2(ptr *C.Vector2Handle, root *bool) *Vector2 {
    if ptr == nil {
        return nil
    }
    return &Vector2{ptr: ptr, root: root}
}

func (v *Vector2) X() int32 {
    if v == nil || v.ptr == nil {
        return 0
    }
    if v.root != nil && *v.root {
        panic("Vector2 handle is closed")
    }
    return int32(C.cgowrap_Vector2_X(v.ptr))
}

func (v *Vector2) Y() int32 {
    if v == nil || v.ptr == nil {
        return 0
    }
    if v.root != nil && *v.root {
        panic("Vector2 handle is closed")
    }
    return int32(C.cgowrap_Vector2_Y(v.ptr))
}

func (v *Vector2) OperBool() bool {
    if v == nil || v.ptr == nil {
        return false
    }
    if v.root != nil && *v.root {
        panic("Vector2 handle is closed")
    }
    result := C.cgowrap_Vector2_OperBool(v.ptr)
    return bool(result)
}

func (v *Vector2) OperPlus(rhs *Vector2) *Vector2 {
    if v == nil || v.ptr == nil {
        return nil
    }
    if v.root != nil && *v.root {
        panic("Vector2 handle is closed")
    }
    var cArg0 *C.Vector2Handle
    if rhs == nil || rhs.ptr == nil {
        panic("Vector2 handle is required but nil")
    }
    if rhs.root != nil && *rhs.root {
        panic("Vector2 handle is closed")
    }
    cArg0 = rhs.ptr
    raw := C.cgowrap_Vector2_OperPlus(v.ptr, cArg0)
    if raw == nil {
        return nil
    }
    return newOwnedVector2(raw)
}

func (v *Vector2) OperEqual(rhs *Vector2) bool {
    if v == nil || v.ptr == nil {
        return false
    }
    if v.root != nil && *v.root {
        panic("Vector2 handle is closed")
    }
    var cArg0 *C.Vector2Handle
    if rhs == nil || rhs.ptr == nil {
        panic("Vector2 handle is required but nil")
    }
    if rhs.root != nil && *rhs.root {
        panic("Vector2 handle is closed")
    }
    cArg0 = rhs.ptr
    result := C.cgowrap_Vector2_OperEqual(v.ptr, cArg0)
    return bool(result)
}
