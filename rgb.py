import numpy as np
import colour

with open("wavelengths.txt") as file:
    wavelengths = np.array(file.readlines()).astype(float)

with open("sbc_flux.txt") as file:
    flux = np.array(file.readlines()).astype(float)

# Convert wavelengths to nanometers (if necessary)
wavelengths_nm = wavelengths / 10

# Load CIE 1931 color-matching functions
cmfs = colour.MSDS_CMFS["CIE 1931 2 Degree Standard Observer"]

# Interpolate your data to match CMFs
flux_interp = np.interp(cmfs.wavelengths, wavelengths_nm, flux)

# Compute XYZ values
XYZ = colour.sd_to_XYZ(colour.SpectralDistribution(flux_interp, cmfs.wavelengths))

# Convert XYZ to sRGB
RGB = colour.XYZ_to_sRGB(XYZ / 100)  # Normalize XYZ if necessary

# Apply gamma correction
RGB = colour.cctf_encoding(RGB)

# Scale to 0-255
# RGB = np.round(RGB * 255).astype(int)

print(RGB)