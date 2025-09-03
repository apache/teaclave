# Apache Teaclave™: Empowering Building Memory Safe Trusted Applications in Confidential Computing

[![License](https://img.shields.io/badge/license-Apache-green.svg)](LICENSE)
[![Homepage](https://img.shields.io/badge/site-homepage-blue)](https://teaclave.apache.org/)

Welcome to the main repository of the **Teaclave** project, an open-source initiative under the Apache Software Foundation. Teaclave empowers developers to build **memory-safe** Trusted Applications across diverse **confidential computing platforms**, including Intel SGX and Arm TrustZone.

Originally built as a general-purpose secure computing framework, Teaclave has evolved into a vibrant ecosystem focused on **SDKs** that enable developers to directly build custom Trusted Applications. This shift has been driven by how the community naturally adopted and extended the project.

## The Teaclave SDK Ecosystem

Teaclave currently maintains SDKs for multiple Trusted Execution Environment (TEE) platforms:

- 🔐 [Teaclave SGX SDK](https://github.com/apache/teaclave-sgx-sdk) — A Rust-based SDK for Intel SGX
- 🔐 [Teaclave TrustZone SDK](https://github.com/apache/teaclave-trustzone-sdk) — A Rust-based SDK for Arm TrustZone
- ☕ [Teaclave Java TEE SDK](https://github.com/apache/teaclave-java-tee-sdk) — An experimental Java SDK for TEEs
- 📦 [Teaclave Dependency Crates](https://github.com/apache/teaclave-crates) — A collection of ported and TEE-tailored Rust dependencies.

These SDKs form the foundation for building secure and reliable TEE-based applications.

## Repository Status

This repository previously hosted the **Teaclave FaaS framework**, a general-purpose confidential computing platform. As the community’s focus has shifted, the FaaS framework is no longer actively maintained.

To preserve the project’s history and recognize early contributions, the legacy codebase remains available in the deprecated `master` (renamed to `legacy`) branch. Going forward, this repository serves as the **central landing page** for the Teaclave ecosystem, offering:

- A project overview and roadmap
- Guidance on SDKs for different TEE platforms
- Showcases demonstrating real-world applications built with Teaclave SDKs
- A unified contribution guide

## Getting Started

To begin building Trusted Applications, head to the SDK repositories linked above. Each includes installation instructions, documentation, and sample code.

You can also visit our [official website](https://teaclave.apache.org) for more information.

## Contributing

Teaclave is developed in the open following [The Apache Way](https://www.apache.org/theapacheway/). We strive to maintain a project that is community-driven and inclusive.

We welcome all forms of contributions. Please refer to our [Contributing Guide](CONTRIBUTING.md) for more information. A big thank-you to all our [contributors](https://teaclave.apache.org/contributors/)!

## Community

- 📬 Join our [mailing list](https://lists.apache.org/list.html?dev@teaclave.apache.org)
- 🐦 Follow us on [Twitter @ApacheTeaclave](https://twitter.com/ApacheTeaclave)
- 🌐 Learn more at [teaclave.apache.org/community](https://teaclave.apache.org/community/)