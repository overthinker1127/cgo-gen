package generated

/*
#include <stdlib.h>
#include "library_math_wrapper.h"
*/
import "C"

import "errors"

func SharedMultiplier(value int32) int32 {
    return int32(C.cgowrap_library_math_shared_multiplier(C.int(value)))
}

func StaticOffset(value int32) int32 {
    return int32(C.cgowrap_library_math_static_offset(C.int(value)))
}

type LibraryMathLibraryMath struct {
    ptr *C.library_mathLibraryMathHandle
    owned bool
    root *bool
}

func NewLibraryMathLibraryMath(base int32) (*LibraryMathLibraryMath, error) {
    ptr := C.cgowrap_library_math_LibraryMath_new(C.int(base))
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedLibraryMathLibraryMath(ptr), nil
}

func (l *LibraryMathLibraryMath) Close() {
    if l == nil || l.ptr == nil {
        return
    }
    if !l.owned {
        return
    }
    if l.root != nil {
        *l.root = true
    }
    C.cgowrap_library_math_LibraryMath_delete(l.ptr)
    l.ptr = nil
}

func newOwnedLibraryMathLibraryMath(ptr *C.library_mathLibraryMathHandle) *LibraryMathLibraryMath {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &LibraryMathLibraryMath{ptr: ptr, owned: true, root: root}
}

func newBorrowedLibraryMathLibraryMath(ptr *C.library_mathLibraryMathHandle, root *bool) *LibraryMathLibraryMath {
    if ptr == nil {
        return nil
    }
    return &LibraryMathLibraryMath{ptr: ptr, root: root}
}

func requireLibraryMathLibraryMathHandle(l *LibraryMathLibraryMath) *C.library_mathLibraryMathHandle {
    if l == nil || l.ptr == nil {
        panic("LibraryMathLibraryMath handle is required but nil")
    }
    if l.root != nil && *l.root {
        panic("LibraryMathLibraryMath handle is closed")
    }
    return l.ptr
}

func optionalLibraryMathLibraryMathHandle(l *LibraryMathLibraryMath) *C.library_mathLibraryMathHandle {
    if l == nil {
        return nil
    }
    if l.root != nil && *l.root {
        panic("LibraryMathLibraryMath handle is closed")
    }
    return l.ptr
}

func (l *LibraryMathLibraryMath) AddStatic(value int32) int32 {
    if l == nil || l.ptr == nil {
        return 0
    }
    if l.root != nil && *l.root {
        panic("LibraryMathLibraryMath handle is closed")
    }
    return int32(C.cgowrap_library_math_LibraryMath_add_static(l.ptr, C.int(value)))
}

func (l *LibraryMathLibraryMath) MultiplyShared(value int32) int32 {
    if l == nil || l.ptr == nil {
        return 0
    }
    if l.root != nil && *l.root {
        panic("LibraryMathLibraryMath handle is closed")
    }
    return int32(C.cgowrap_library_math_LibraryMath_multiply_shared(l.ptr, C.int(value)))
}
