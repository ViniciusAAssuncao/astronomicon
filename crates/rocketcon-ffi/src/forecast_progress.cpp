#include "rocketcon_forecast_progress.hpp"
#include "rocketcon-ffi/src/lib.rs.h"
#include <stdexcept>
#include <utility>

rocketcon::ForecastProgress::ForecastProgress(
    std::function<void(rocketcon::OrbitPreview, std::uint64_t)> callback
) : callback_(std::move(callback)) {}

void rocketcon::ForecastProgress::publish(
    rocketcon::OrbitPreview preview, std::uint64_t generation
) noexcept {
    if (failure_) return;
    try {
        callback_(std::move(preview), generation);
    } catch (...) {
        failure_ = std::current_exception();
    }
}

void rocketcon::ForecastProgress::rethrow_if_failed() const {
    if (!failure_) return;
    try {
        std::rethrow_exception(failure_);
    } catch (const std::exception&) {
        throw;
    } catch (...) {
        throw std::runtime_error("forecast progress callback failed");
    }
}
