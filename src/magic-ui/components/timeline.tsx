import * as React from "react"
import { Check } from "lucide-react"

import { cn } from "@/lib/utils"

export interface TimelineItem {
  title: string
  description?: string
  date?: string
  icon?: React.ReactNode
  status?: "completed" | "current" | "upcoming"
}

export interface TimelineProps extends React.HTMLAttributes<HTMLDivElement> {
  items: TimelineItem[]
  orientation?: "vertical" | "horizontal"
}

const Timeline = React.forwardRef<HTMLDivElement, TimelineProps>(
  ({ className, items, orientation = "vertical", ...props }, ref) => {
    if (orientation === "horizontal") {
      return (
        <div ref={ref} className={cn("timeline-horizontal", className)} {...props}>
          {items.map((item, index) => (
            <div key={index} className="timeline-item">
              <div className="timeline-rail">
                {index > 0 ? (
                  <div
                    className={cn(
                      "timeline-line",
                      item.status !== "upcoming" && "timeline-line-active"
                    )}
                  />
                ) : null}

                <div
                  className={cn(
                    "timeline-node",
                    item.status === "completed" && "timeline-node-completed",
                    item.status === "current" && "timeline-node-current",
                    item.status === "upcoming" && "timeline-node-upcoming"
                  )}
                >
                  {item.icon ??
                    (item.status === "completed" ? <Check className="h-4 w-4" /> : <span>{index + 1}</span>)}
                </div>

                {index < items.length - 1 ? (
                  <div
                    className={cn(
                      "timeline-line",
                      !(
                        item.status === "upcoming" ||
                        items[index + 1]?.status === "upcoming"
                      ) && "timeline-line-active"
                    )}
                  />
                ) : null}
              </div>

              <div className="timeline-content">
                <p className="timeline-title">{item.title}</p>
                {item.description ? <p className="timeline-desc">{item.description}</p> : null}
                {item.date ? <p className="timeline-date">{item.date}</p> : null}
              </div>
            </div>
          ))}
        </div>
      )
    }

    return (
      <div ref={ref} className={cn("timeline-vertical", className)} {...props}>
        {items.map((item, index) => (
          <div key={index} className="timeline-item">
            <div className="timeline-stem">
              <div
                className={cn(
                  "timeline-node",
                  item.status === "completed" && "timeline-node-completed",
                  item.status === "current" && "timeline-node-current",
                  item.status === "upcoming" && "timeline-node-upcoming"
                )}
              >
                {item.icon ??
                  (item.status === "completed" ? <Check className="h-4 w-4" /> : <span>{index + 1}</span>)}
              </div>

              {index < items.length - 1 ? (
                <div
                  className={cn(
                    "timeline-line",
                    item.status !== "upcoming" && "timeline-line-active"
                  )}
                />
              ) : null}
            </div>

            <div className="timeline-content">
              <div className="flex items-center gap-2">
                <p className="timeline-title">{item.title}</p>
                {item.date ? <span className="timeline-date">{item.date}</span> : null}
              </div>
              {item.description ? <p className="timeline-desc">{item.description}</p> : null}
            </div>
          </div>
        ))}
      </div>
    )
  }
)
Timeline.displayName = "Timeline"

export { Timeline }
