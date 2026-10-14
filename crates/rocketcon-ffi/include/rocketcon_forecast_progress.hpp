#ifndef ROCKETCON_FORECAST_PROGRESS_HPP
#define ROCKETCON_FORECAST_PROGRESS_HPP

#include <cstdint>
#include <exception>
#include <functional>

namespace rocketcon {
struct OrbitPreview;

class ForecastProgress {
public:
    explicit ForecastProgress(std::function<void(OrbitPreview, std::uint64_t)> callback);
    void publish(OrbitPreview preview, std::uint64_t generation) noexcept;
    void rethrow_if_failed() const;

private:
    std::function<void(OrbitPreview, std::uint64_t)> callback_;
    std::exception_ptr failure_;
};
}

#endif
