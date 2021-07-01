import React from 'react'
import clsx from 'clsx'
import { BackgroundColorSpectrum } from '../../tailwind'
import { semigroupAll } from 'fp-ts/lib/Semigroup'

interface Props {
  color?: BackgroundColorSpectrum
  working?: boolean
  pinging?: boolean
  pingColor?: BackgroundColorSpectrum
  small?: boolean
  icon?: JSX.Element
}

const E: React.FC<Props & React.ButtonHTMLAttributes<HTMLButtonElement>> = ({
  color,
  pingColor,
  children,
  working,
  className,
  disabled,
  pinging,
  small,
  icon,
  ...props
}) => {
  const colorSpectrum = color || 'gray'
  const bgColor = `${colorSpectrum}-200`
  const hvColor = `${colorSpectrum}-300`
  const dsColor = `${colorSpectrum}-100`
  const pgColor = `${pingColor || colorSpectrum}-300`
  disabled = disabled || working
  return (
    <span className={clsx('relative inline-flex rounded-md', className)}>
      <button
        className={clsx(
          'inline-flex items-center leading-6 font-medium rounded-md transition ease-in-out duration-150 focus:outline-none',
          [!disabled && `hover:bg-${hvColor} bg-${bgColor}`],
          [disabled && `cursor-not-allowed bg-${dsColor}`],
          {
            'px-4 py-2 text-base': !small,
            'px-2 py-1 text-sm': small,
          },
        )}
        disabled={disabled}
        {...props}
      >
        {!working && icon && <div className="mr-2">{icon}</div>}
        {working && (
          <svg
            className="animate-spin -ml-1 mr-3 h-5 w-5"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
          >
            <circle
              className="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              strokeWidth="4"
            ></circle>
            <path
              className="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            ></path>
          </svg>
        )}
        {children}
      </button>
      {pinging && (
        <div className="flex absolute top-0 right-0 -mt-0.5 -mr-1">
          <span className="absolute inline-flex animate-ping">
            <span className={`inline-flex rounded-full h-3 w-3 bg-${pgColor} opacity-75`}></span>
          </span>
          <span className={`relative inline-flex rounded-full h-3 w-3 bg-${pgColor}`}></span>
        </div>
      )}
    </span>
  )
}

export default E
