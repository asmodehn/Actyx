import React from 'react'
import styled from 'styled-components'

const IconStyledBlue = styled.span`
  svg {
    path {
      fill: #1998ff;
    }
    height: 21px;
    margin-right: 10px;
    padding-top: 3px;
  }
`

const IconStyledGray = styled.span`
  svg {
    path {
      fill: #586069;
    }
    height: 18px;
    margin-right: 10px;
    padding-top: 3px;
  }
`

const IconStyledLightGray = styled.span`
  svg {
    path {
      fill: #ebedf0;
    }
    height: 18px;
    margin-right: 10px;
    padding-top: 3px;
  }
`
export const Calendar = () => (
  <IconStyledGray>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
      <path d="M197.18,37.76H195.5V10.24a10,10,0,0,0-20,0V37.76h-95V10.24a10,10,0,0,0-20,0V37.76H58.82A58.83,58.83,0,0,0,0,96.59V196.94a58.82,58.82,0,0,0,58.82,58.82H197.18A58.82,58.82,0,0,0,256,196.94V96.59A58.83,58.83,0,0,0,197.18,37.76Zm-138.36,20H60.5v12a10,10,0,0,0,20,0v-12h95v12a10,10,0,0,0,20,0v-12h1.68A38.87,38.87,0,0,1,236,96.59v14.17H20V96.59A38.87,38.87,0,0,1,58.82,57.76Zm138.36,178H58.82A38.86,38.86,0,0,1,20,196.94V130.76H236v66.18A38.86,38.86,0,0,1,197.18,235.76Z" />
    </svg>
  </IconStyledGray>
)

export const Commit = () => (
  <IconStyledGray>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
      <path d="M243,118H187.15a60,60,0,0,0-118.3,0H13a10,10,0,0,0,0,20H68.85a60,60,0,0,0,118.3,0H243a10,10,0,0,0,0-20ZM128,168a40,40,0,1,1,40-40A40,40,0,0,1,128,168Z" />
    </svg>
  </IconStyledGray>
)

export const Laptop = () => (
  <IconStyledBlue>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
      <path d="M214.79,58A3.21,3.21,0,0,1,218,61.21V161H38V61.21A3.21,3.21,0,0,1,41.21,58H214.79m0-20H41.21A23.21,23.21,0,0,0,18,61.21V181H238V61.21A23.21,23.21,0,0,0,214.79,38Z" />
      <path d="M256,188H0v12a17,17,0,0,0,17,17H239a17,17,0,0,0,17-17V188Z" />
    </svg>
  </IconStyledBlue>
)

export const Arrow = () => (
  <IconStyledLightGray>
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
      <path d="M46,189.1a13.7,13.7,0,0,0,2.3.17H179.84L177,190.6a27.42,27.42,0,0,0-7.54,5.34l-36.91,36.91a13.77,13.77,0,0,0-1.92,17.68,13.3,13.3,0,0,0,18.64,2.87,12.47,12.47,0,0,0,1.38-1.16l66.72-66.72a13.37,13.37,0,0,0,0-18.89L150.61,99.9a13.32,13.32,0,0,0-18.89,0,10.54,10.54,0,0,0-1.13,1.29,13.82,13.82,0,0,0,1.92,17.68l36.82,37a27.66,27.66,0,0,0,6.67,4.88l4,1.79H49a13.85,13.85,0,0,0-14.1,11.22A13.36,13.36,0,0,0,46,189.1Z" />
    </svg>
  </IconStyledLightGray>
)
