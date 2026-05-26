#include "score_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "score.h"

int cgowrap_score_delta(int current, int previous) {
    return score_delta(current, previous);
}

int cgowrap_score_total(int wins, int draws) {
    return score_total(wins, draws);
}
