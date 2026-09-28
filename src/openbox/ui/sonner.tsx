import type { CSSProperties } from "react";
import {
  ArrowPathIcon,
  CheckCircleIcon,
  ExclamationTriangleIcon,
  InformationCircleIcon,
  XCircleIcon,
} from "@heroicons/react/24/outline";
import { Toaster as Sonner, type ToasterProps } from "sonner";

export function Toaster({ className, position = "top-center", style, toastOptions, ...props }: ToasterProps) {
  return <Sonner
    className={["openbox-toaster", className].filter(Boolean).join(" ")}
    position={position}
    closeButton
    richColors
    icons={{
      success: <span className="openbox-sonner-icon"><CheckCircleIcon /></span>,
      info: <span className="openbox-sonner-icon"><InformationCircleIcon /></span>,
      warning: <span className="openbox-sonner-icon"><ExclamationTriangleIcon /></span>,
      error: <span className="openbox-sonner-icon"><XCircleIcon /></span>,
      loading: <span className="openbox-sonner-icon"><ArrowPathIcon className="spinning" /></span>,
    }}
    style={{
      "--width": "min(380px, calc(100vw - 32px))",
      "--border-radius": "14px",
      ...style,
    } as CSSProperties}
    toastOptions={{
      ...toastOptions,
      classNames: {
        toast: "openbox-sonner-toast",
        title: "openbox-sonner-title",
        description: "openbox-sonner-description",
        icon: "openbox-sonner-icon-slot",
        closeButton: "openbox-sonner-close",
        ...toastOptions?.classNames,
      },
    }}
    {...props}
  />;
}
