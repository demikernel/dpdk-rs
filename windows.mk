# Copyright (c) Microsoft Corporation.
# Licensed under the MIT license.

#=======================================================================================================================
# Environment Variables
#=======================================================================================================================

PREFIX = $(USERPROFILE)
!if [set LIBDPDK_PATH=$(PREFIX)\AppData\Local\dpdk]
!endif

#=======================================================================================================================
# Tools
#=======================================================================================================================

CARGO = cargo
RM = del
!if [set CC=clang]
!endif

#=======================================================================================================================
# Switches
#=======================================================================================================================

# Set build mode.
!ifndef DEBUG
BUILD = release
!else
BUILD = dev
!endif

# Set build flags.
FLAGS = $(FLAGS) --profile $(BUILD) --features=mlx4
