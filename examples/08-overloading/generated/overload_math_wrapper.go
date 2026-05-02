package generated

/*
#include <stdlib.h>
#include "overload_math_wrapper.h"
*/
import "C"

import "errors"

import "fmt"

type OverloadMath struct {
    ptr *C.OverloadMathHandle
    owned bool
    root *bool
}

func NewOverloadMath() (*OverloadMath, error) {
    ptr := C.cgowrap_OverloadMath_new__void()
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedOverloadMath(ptr), nil
}

func NewOverloadMathWithBase(base int32) (*OverloadMath, error) {
    ptr := C.cgowrap_OverloadMath_new__int(C.int(base))
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedOverloadMath(ptr), nil
}

func (o *OverloadMath) Close() {
    if o == nil || o.ptr == nil {
        return
    }
    if !o.owned {
        return
    }
    if o.root != nil {
        *o.root = true
    }
    C.cgowrap_OverloadMath_delete(o.ptr)
    o.ptr = nil
}

func newOwnedOverloadMath(ptr *C.OverloadMathHandle) *OverloadMath {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &OverloadMath{ptr: ptr, owned: true, root: root}
}

func newBorrowedOverloadMath(ptr *C.OverloadMathHandle, root *bool) *OverloadMath {
    if ptr == nil {
        return nil
    }
    return &OverloadMath{ptr: ptr, root: root}
}

func requireOverloadMathHandle(o *OverloadMath) *C.OverloadMathHandle {
    if o == nil || o.ptr == nil {
        panic("OverloadMath handle is required but nil")
    }
    if o.root != nil && *o.root {
        panic("OverloadMath handle is closed")
    }
    return o.ptr
}

func optionalOverloadMathHandle(o *OverloadMath) *C.OverloadMathHandle {
    if o == nil {
        return nil
    }
    if o.root != nil && *o.root {
        panic("OverloadMath handle is closed")
    }
    return o.ptr
}

func (o *OverloadMath) AddInt32Int32(lhs int32, rhs int32) float64 {
    if o == nil || o.ptr == nil {
        return 0
    }
    if o.root != nil && *o.root {
        panic("OverloadMath handle is closed")
    }
    return float64(C.cgowrap_OverloadMath_Add__int_int_mut(o.ptr, C.int(lhs), C.int(rhs)))
}

func (o *OverloadMath) AddFloat64Float64(lhs float64, rhs float64) float64 {
    if o == nil || o.ptr == nil {
        return 0
    }
    if o.root != nil && *o.root {
        panic("OverloadMath handle is closed")
    }
    return float64(C.cgowrap_OverloadMath_Add__double_double_mut(o.ptr, C.double(lhs), C.double(rhs)))
}

func (o *OverloadMath) Add(args ...any) (float64, error) {
    if o == nil || o.ptr == nil {
        return 0, fmt.Errorf("OverloadMath receiver is nil")
    }
    switch len(args) {
    case 2:
        {
            arg0, ok0 := args[0].(float64)
            arg1, ok1 := args[1].(float64)
            if ok0 && ok1 {
                return o.AddFloat64Float64(arg0, arg1), nil
            }
        }
        {
            arg0, ok0 := args[0].(int32)
            arg1, ok1 := args[1].(int32)
            if ok0 && ok1 {
                return o.AddInt32Int32(arg0, arg1), nil
            }
        }
    }
    return 0, fmt.Errorf("no matching overload for OverloadMath.Add")
}
