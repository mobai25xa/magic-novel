import * as React from "react"

import { cn } from "@/lib/utils"

type TabsContextValue = {
  value: string
  setValue: (value: string) => void
}

const TabsContext = React.createContext<TabsContextValue | null>(null)

export interface TabsProps extends React.HTMLAttributes<HTMLDivElement> {
  value?: string
  defaultValue?: string
  onValueChange?: (value: string) => void
  orientation?: "horizontal" | "vertical"
  children: React.ReactNode
}

export interface TabProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  value: string
  children: React.ReactNode
}

export interface TabPanelProps extends React.HTMLAttributes<HTMLDivElement> {
  value: string
  children: React.ReactNode
}

function isTabElement(child: React.ReactNode): child is React.ReactElement<TabProps> {
  return React.isValidElement(child) && child.type === Tab
}

const Tabs = React.forwardRef<HTMLDivElement, TabsProps>(
  ({ className, children, value, defaultValue, onValueChange, orientation = "horizontal", ...props }, ref) => {
    const [uncontrolledValue, setUncontrolledValue] = React.useState(defaultValue ?? "")

    const childArray = React.Children.toArray(children)
    const tabChildren = childArray.filter(isTabElement)
    const panelChildren = childArray.filter((child) => !isTabElement(child))
    const firstTabValue = tabChildren[0]?.props.value

    React.useEffect(() => {
      if (value === undefined && !uncontrolledValue && firstTabValue) {
        setUncontrolledValue(firstTabValue)
      }
    }, [value, uncontrolledValue, firstTabValue])

    const currentValue = value ?? uncontrolledValue

    const setValue = React.useCallback(
      (next: string) => {
        if (value === undefined) {
          setUncontrolledValue(next)
        }
        onValueChange?.(next)
      },
      [value, onValueChange]
    )

    return (
      <TabsContext.Provider value={{ value: currentValue, setValue }}>
        <div ref={ref} className={className} {...props}>
          <div className={cn("tabs-container", orientation === "vertical" && "tabs-vertical")} role="tablist">
            {tabChildren}
          </div>
          {panelChildren}
        </div>
      </TabsContext.Provider>
    )
  }
)
Tabs.displayName = "Tabs"

const Tab = React.forwardRef<HTMLButtonElement, TabProps>(
  ({ className, value, onClick, children, ...props }, ref) => {
    const context = React.useContext(TabsContext)
    const active = context?.value === value

    return (
      <button
        ref={ref}
        type="button"
        role="tab"
        aria-selected={active}
        className={cn("tab", active && "active", className)}
        onClick={(event) => {
          onClick?.(event)
          if (!event.defaultPrevented) {
            context?.setValue(value)
          }
        }}
        {...props}
      >
        {children}
      </button>
    )
  }
)
Tab.displayName = "Tab"

const TabPanel = React.forwardRef<HTMLDivElement, TabPanelProps>(
  ({ value, className, children, ...props }, ref) => {
    const context = React.useContext(TabsContext)

    if (!context || context.value !== value) {
      return null
    }

    return (
      <div ref={ref} role="tabpanel" className={className} {...props}>
        {children}
      </div>
    )
  }
)
TabPanel.displayName = "TabPanel"

export { Tabs, Tab, TabPanel }
