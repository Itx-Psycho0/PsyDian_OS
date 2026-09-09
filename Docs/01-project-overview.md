# Psydian_OS :- Complete Project Documentation

**Track:** Systems / Generative AI  
**Domain:** Operating Systems / Artificial Intelligence  
**Duration:** 8 weeks  
**Team:** 2 students  
**Level:** Intermediate  
**Stack:** Rust, `no_std`, bootloader, x86_64, QEMU, UART/Serial, Host-side AI Bridge, REST/HTTPS AI APIs  
**Constraint:** 8-week development timeline and bare-metal OS development complexity

## Project Description

Designing and implementing Psydian, an AI-assisted operating-system prototype built primarily in Rust for the x86_64 architecture. The project focuses on developing a minimal bare-metal kernel that runs in QEMU, provides a command-line shell and system diagnostics, and connects to an external AI service through a host-side AI Bridge.

The system is designed to assist users with system interaction, troubleshooting, and diagnostic analysis. Core operating-system functionality remains independent of the AI layer, while AI-generated responses are treated as untrusted suggestions and potentially privileged actions require normal command validation and user confirmation.

# Psydian_OS - Project Overview

## Project Identity

- **Project Name:** PsyDian_OS
- **Project Type:** AI-Assisted Operating System Prototype
- **Track:** Systems / Generative AI
- **Domain:** Operating Systems / Artificial Intelligence
- **Duration:** 8 weeks
- **Team:** 2 students
- **Skill Level:** Intermediate
- **Primary Users:** Developers, technical users, power users, students, and users interested in AI-assisted command-line interaction
- **Primary Platform:** x86_64
- **Development Environment:** Linux + QEMU
- **Constraint:** Limited project duration and bare-metal OS development complexity

## Executive Summary

Psydian is an AI-assisted operating-system prototype written primarily in Rust. The project aims to build a minimal x86_64 operating-system environment with a custom kernel, command-line shell, system diagnostics, and an external AI assistance layer.

The core idea is to make system interaction more intelligent by allowing users to request explanations, troubleshooting assistance, and diagnostic analysis through a familiar command-line interface. Instead of placing AI functionality directly inside the privileged kernel, Psydian uses a host-side AI Bridge that communicates with the operating system through a controlled communication channel and connects to an external AI service through the host system's network stack.

The project is designed as a decision-support and assistance system rather than an autonomous system. AI-generated responses are treated as untrusted suggestions, while command validation and user confirmation remain responsible for preventing unsafe or unintended operations.

## Core System Flow

User → Psydian Shell → Psydian Kernel → System Diagnostics → Serial / Diagnostic Channel → Host-side AI Bridge → External AI Service → AI Response → AI Bridge → Psydian → Shell → User

For normal system operations, the AI layer is not required:

User → Psydian Shell → Psydian Kernel → Kernel Subsystem → Result → User

## MVP

1. Bootable x86_64 Psydian kernel running in QEMU.
2. Rust-based bare-metal kernel foundation.
3. Kernel serial communication.
4. Kernel logging and diagnostic output.
5. CPU exception and interrupt handling required by the MVP.
6. Keyboard input handling.
7. Basic memory-management foundation.
8. Interactive command-line shell.
9. Structured kernel diagnostic records.
10. Host-side AI Bridge.
11. Communication between Psydian and the AI Bridge.
12. External AI API integration through the host system.
13. AI-assisted system diagnostics and explanations.
14. Validation of AI-generated responses and suggested commands.
15. Explicit user confirmation for privileged or destructive actions.

## Out of Scope for MVP

- Full general-purpose operating-system functionality.
- Native networking stack inside the kernel.
- Full filesystem implementation.
- Production-grade hardware-driver ecosystem.
- Multi-user account management.
- Autonomous execution of arbitrary AI-generated commands.
- Direct external network/API access from the bare-metal kernel.
- Training a large language model from scratch.
- Production-grade AI infrastructure.
- Advanced voice interaction if it cannot be completed within the 8-week timeline.
- Full desktop GUI environment.
- Production deployment on physical hardware as the primary target.