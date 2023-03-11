library(magrittr)
library(ggplot2)

logs <- jsonlite::stream_in(file("logs.ndjson"), verbose = FALSE)

parse_duration <- function(s) {
  n <- as.numeric(substr(s, 1, nchar(s) - 2))
  ifelse(endsWith(s, "ms"), n, n / 1000)
}

is_outlier <- function(ms) {
  # outliers are greater than 10ms
  return(ms > 10)
}

data <- dplyr::bind_rows(
  phase = logs$span$name,
  file = logs$span$source_name,
  duration_ms = parse_duration(logs$fields$time.busy)
) %>%
  dplyr::filter(
    phase %in% c("parse_cst", "check_module", "generate_javascript")
  ) %>%
  dplyr::mutate(
    phase = dplyr::recode(phase,
      parse_cst = "Parse",
      check_module = "Typecheck",
      generate_javascript = "Codegen"
    ),
    outlier = ifelse(is_outlier(duration_ms), file, as.numeric(NA))
  )

plot <- ggplot(
  data,
  aes(
    x = factor(phase, levels = c("Parse", "Typecheck", "Codegen")),
    y = duration_ms
  )
) +
  geom_boxplot() +
  geom_text(aes(label = outlier), na.rm = TRUE, hjust = -0.1, size = 3) +
  xlab("Phase") +
  ylab("Time (milliseconds)")

ggsave("plot.png", plot = plot)
