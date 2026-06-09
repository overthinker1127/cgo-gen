package generated

/*
#include <stdlib.h>
#include "default_arguments_wrapper.h"
*/
import "C"

import "errors"

import "fmt"

func Clamp(args ...any) (int32, error) {
    switch len(args) {
    case 1:
        {
            arg0, ok0 := args[0].(int32)
            if ok0 {
                result := C.cgowrap_Clamp__int(C.int(arg0))
                return int32(result), nil
            }
        }
    case 2:
        {
            arg0, ok0 := args[0].(int32)
            arg1, ok1 := args[1].(int32)
            if ok0 && ok1 {
                result := C.cgowrap_Clamp__int_int(C.int(arg0), C.int(arg1))
                return int32(result), nil
            }
        }
    }
    return 0, fmt.Errorf("no matching overload for Clamp")
}

type DefaultCounter struct {
    ptr *C.DefaultCounterHandle
    owned bool
    root *bool
}

func NewDefaultCounterWithStart(start int32) (*DefaultCounter, error) {
    ptr := C.cgowrap_DefaultCounter_new__int(C.int(start))
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedDefaultCounter(ptr), nil
}

func NewDefaultCounter() (*DefaultCounter, error) {
    ptr := C.cgowrap_DefaultCounter_new__void()
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedDefaultCounter(ptr), nil
}

func (d *DefaultCounter) Close() {
    if d == nil || d.ptr == nil {
        return
    }
    if !d.owned {
        return
    }
    if d.root != nil {
        *d.root = true
    }
    C.cgowrap_DefaultCounter_delete(d.ptr)
    d.ptr = nil
}

func newOwnedDefaultCounter(ptr *C.DefaultCounterHandle) *DefaultCounter {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &DefaultCounter{ptr: ptr, owned: true, root: root}
}

func newBorrowedDefaultCounter(ptr *C.DefaultCounterHandle, root *bool) *DefaultCounter {
    if ptr == nil {
        return nil
    }
    return &DefaultCounter{ptr: ptr, root: root}
}

func (d *DefaultCounter) Value() int32 {
    if d == nil || d.ptr == nil {
        return 0
    }
    if d.root != nil && *d.root {
        panic("DefaultCounter handle is closed")
    }
    return int32(C.cgowrap_DefaultCounter_Value(d.ptr))
}

func (d *DefaultCounter) Add(args ...any) (int32, error) {
    if d == nil || d.ptr == nil {
        return 0, fmt.Errorf("DefaultCounter receiver is nil")
    }
    if d.root != nil && *d.root {
        panic("DefaultCounter handle is closed")
    }
    switch len(args) {
    case 1:
        {
            arg0, ok0 := args[0].(int32)
            if ok0 {
                result := C.cgowrap_DefaultCounter_Add__int_mut(d.ptr, C.int(arg0))
                return int32(result), nil
            }
        }
    case 2:
        {
            arg0, ok0 := args[0].(int32)
            arg1, ok1 := args[1].(int32)
            if ok0 && ok1 {
                result := C.cgowrap_DefaultCounter_Add__int_int_mut(d.ptr, C.int(arg0), C.int(arg1))
                return int32(result), nil
            }
        }
    }
    return 0, fmt.Errorf("no matching overload for DefaultCounter.Add")
}
