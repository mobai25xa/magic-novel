import * as React from "react"
import { AlertCircle, AlertTriangle, CheckCircle, Info, X } from "lucide-react"

import { cn } from "@/lib/utils"

type ToastVariant = "default" | "success" | "warning" | "destructive" | "info"

const DEFAULT_TOAST_DURATION = 5000
const DISMISS_ANIMATION_MS = 220

export interface ToastProps {
  id: string
  title?: string
  description?: string
  variant?: ToastVariant
  duration?: number
  action?: React.ReactNode
}

export interface InternalToast extends ToastProps {
  dismissing?: boolean
}

export interface ToastContextType {
  toasts: InternalToast[]
  addToast: (toast: Omit<ToastProps, "id">) => string
  removeToast: (id: string) => void
}

const ToastContext = React.createContext<ToastContextType | undefined>(undefined)

const toastIconMap: Record<ToastVariant, React.ComponentType<{ className?: string }>> = {
  default: Info,
  info: Info,
  success: CheckCircle,
  warning: AlertTriangle,
  destructive: AlertCircle,
}

const toastVariantClassMap: Record<ToastVariant, string> = {
  default: "toast-default",
  info: "toast-info",
  success: "toast-success",
  warning: "toast-warning",
  destructive: "toast-destructive",
}

const createToastId = () => {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID()
  }

  return Math.random().toString(36).slice(2, 10)
}

export interface ToastItemProps extends InternalToast {
  onClose: (id: string) => void
}

const Toast = ({
  id,
  title,
  description,
  variant = "default",
  duration = DEFAULT_TOAST_DURATION,
  action,
  dismissing,
  onClose,
}: ToastItemProps) => {
  const [progress, setProgress] = React.useState(100)
  const Icon = toastIconMap[variant]

  React.useEffect(() => {
    if (duration === 0 || dismissing) {
      return
    }

    const frame = requestAnimationFrame(() => {
      setProgress(0)
    })

    return () => cancelAnimationFrame(frame)
  }, [duration, dismissing])

  return (
    <div className={cn("toast", toastVariantClassMap[variant], dismissing && "toast-dismissing")}>
      <Icon className="toast-icon" />
      <div className="toast-body">
        {title && <p className="toast-title">{title}</p>}
        {description && <p className={cn("toast-desc", title && "has-title")}>{description}</p>}
        {action ? <div className="toast-action">{action}</div> : null}
      </div>
      <button type="button" onClick={() => onClose(id)} className="toast-close" aria-label="Close">
        <X className="toast-close-icon" />
      </button>
      {duration !== 0 ? (
        <div
          className="toast-progress"
          style={{
            width: `${progress}%`,
            transitionDuration: `${duration}ms`,
          }}
        />
      ) : null}
    </div>
  )
}

const ToastContainer = ({
  toasts,
  removeToast,
}: {
  toasts: InternalToast[]
  removeToast: (id: string) => void
}) => {
  if (toasts.length === 0) {
    return null
  }

  return (
    <div className="toast-container">
      {toasts.map((currentToast) => (
        <Toast key={currentToast.id} {...currentToast} onClose={removeToast} />
      ))}
    </div>
  )
}

const useToast = (): ToastContextType => {
  const context = React.useContext(ToastContext)

  if (!context) {
    throw new Error("useToast must be used within a ToastProvider")
  }

  return context
}

const ToastProvider = ({ children }: { children: React.ReactNode }) => {
  const [toasts, setToasts] = React.useState<InternalToast[]>([])
  const autoDismissTimers = React.useRef(new Map<string, ReturnType<typeof setTimeout>>())
  const removeTimers = React.useRef(new Map<string, ReturnType<typeof setTimeout>>())

  const clearToastTimers = React.useCallback((id: string) => {
    const autoDismissTimer = autoDismissTimers.current.get(id)
    if (autoDismissTimer) {
      clearTimeout(autoDismissTimer)
      autoDismissTimers.current.delete(id)
    }

    const removeTimer = removeTimers.current.get(id)
    if (removeTimer) {
      clearTimeout(removeTimer)
      removeTimers.current.delete(id)
    }
  }, [])

  const removeToastNow = React.useCallback(
    (id: string) => {
      clearToastTimers(id)
      setToasts((previousToasts) => previousToasts.filter((toastItem) => toastItem.id !== id))
    },
    [clearToastTimers]
  )

  const removeToast = React.useCallback(
    (id: string) => {
      clearToastTimers(id)
      setToasts((previousToasts) =>
        previousToasts.map((toastItem) =>
          toastItem.id === id ? { ...toastItem, dismissing: true } : toastItem
        )
      )

      const timer = setTimeout(() => {
        removeToastNow(id)
      }, DISMISS_ANIMATION_MS)

      removeTimers.current.set(id, timer)
    },
    [clearToastTimers, removeToastNow]
  )

  const addToast = React.useCallback(
    (toastInput: Omit<ToastProps, "id">) => {
      const id = createToastId()
      const duration = toastInput.duration ?? DEFAULT_TOAST_DURATION
      const nextToast: InternalToast = {
        id,
        title: toastInput.title,
        description: toastInput.description,
        variant: toastInput.variant ?? "default",
        duration,
        action: toastInput.action,
      }

      setToasts((previousToasts) => [...previousToasts, nextToast])

      if (duration !== 0) {
        const timer = setTimeout(() => {
          removeToast(id)
        }, duration)

        autoDismissTimers.current.set(id, timer)
      }

      return id
    },
    [removeToast]
  )

  React.useEffect(() => {
    toast._addToast = addToast

    return () => {
      if (toast._addToast === addToast) {
        toast._addToast = null
      }
    }
  }, [addToast])

  React.useEffect(() => {
    const autoDismissTimersRef = autoDismissTimers.current
    const removeTimersRef = removeTimers.current

    return () => {
      autoDismissTimersRef.forEach((timer) => clearTimeout(timer))
      autoDismissTimersRef.clear()
      removeTimersRef.forEach((timer) => clearTimeout(timer))
      removeTimersRef.clear()
    }
  }, [])

  return (
    <ToastContext.Provider value={{ toasts, addToast, removeToast }}>
      {children}
      <ToastContainer toasts={toasts} removeToast={removeToast} />
    </ToastContext.Provider>
  )
}

const toast = {
  _addToast: null as ((toastItem: Omit<ToastProps, "id">) => string) | null,

  success: (title: string, description?: string) => {
    toast._addToast?.({ title, description, variant: "success" })
  },
  error: (title: string, description?: string) => {
    toast._addToast?.({ title, description, variant: "destructive" })
  },
  warning: (title: string, description?: string) => {
    toast._addToast?.({ title, description, variant: "warning" })
  },
  info: (title: string, description?: string) => {
    toast._addToast?.({ title, description, variant: "info" })
  },
  default: (title: string, description?: string) => {
    toast._addToast?.({ title, description, variant: "default" })
  },
}

export { Toast, ToastContainer, ToastProvider, useToast, toast }
