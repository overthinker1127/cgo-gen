package generated

/*
#include <stdlib.h>
#include "score_wrapper.h"
*/
import "C"

func ScoreDelta(current int32, previous int32) int32 {
    return int32(C.cgowrap_score_delta(C.int(current), C.int(previous)))
}

func ScoreTotal(wins int32, draws int32) int32 {
    return int32(C.cgowrap_score_total(C.int(wins), C.int(draws)))
}
