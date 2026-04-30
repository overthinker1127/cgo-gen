#include "score.h"

int score_total(int wins, int draws) {
    return wins * 3 + draws;
}

int score_delta(int current, int previous) {
    return current - previous;
}
