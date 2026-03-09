import * as React from "react"

import { cn } from "@/lib/utils"

export interface DataTableProps extends React.TableHTMLAttributes<HTMLTableElement> {
  children: React.ReactNode
  containerClassName?: string
}

export interface StatusDotProps extends React.HTMLAttributes<HTMLSpanElement> {
  status: "active" | "inactive"
}

const DataTable = React.forwardRef<HTMLTableElement, DataTableProps>(
  ({ className, containerClassName, children, ...props }, ref) => {
    return (
      <div className={cn("data-table-container", containerClassName)}>
        <table ref={ref} className={cn("data-table", className)} {...props}>
          {children}
        </table>
      </div>
    )
  }
)
DataTable.displayName = "DataTable"

const StatusDot = React.forwardRef<HTMLSpanElement, StatusDotProps>(
  ({ className, status, ...props }, ref) => {
    return (
      <span
        ref={ref}
        className={cn("status-dot", status === "active" ? "status-active" : "status-inactive", className)}
        {...props}
      />
    )
  }
)
StatusDot.displayName = "StatusDot"

export { DataTable, StatusDot }
