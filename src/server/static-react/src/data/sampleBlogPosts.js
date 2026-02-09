/**
 * Blog post sample data generator for the Ingestion tab.
 * Generates 100 randomized blog posts with realistic metadata.
 */

const AUTHORS = [
  "Sarah Chen",
  "Michael Rodriguez",
  "Emily Johnson",
  "David Kim",
  "Lisa Wang",
  "James Thompson",
  "Maria Garcia",
  "Alex Chen",
  "Rachel Green",
  "Tom Wilson",
  "Jennifer Lee",
  "Chris Anderson",
  "Amanda Taylor",
  "Ryan Murphy",
  "Jessica Brown",
  "Kevin Park",
  "Nicole Davis",
  "Brandon White",
  "Stephanie Martinez",
  "Daniel Liu",
];

const TOPICS = [
  "Technology",
  "Programming",
  "Web Development",
  "Data Science",
  "Machine Learning",
  "Artificial Intelligence",
  "Cloud Computing",
  "DevOps",
  "Cybersecurity",
  "Mobile Development",
  "UI/UX Design",
  "Product Management",
  "Startup Life",
  "Career Advice",
  "Industry Trends",
  "Open Source",
  "Software Architecture",
  "Database Design",
  "API Development",
  "Testing",
];

const TAG_SETS = [
  ["javascript", "webdev", "tutorial"],
  ["python", "datascience", "ai"],
  ["react", "frontend", "javascript"],
  ["nodejs", "backend", "api"],
  ["docker", "devops", "deployment"],
  ["aws", "cloud", "infrastructure"],
  ["machine-learning", "python", "data"],
  ["typescript", "webdev", "frontend"],
  ["kubernetes", "devops", "containers"],
  ["sql", "database", "backend"],
  ["git", "version-control", "workflow"],
  ["testing", "quality", "tdd"],
  ["security", "cybersecurity", "best-practices"],
  ["performance", "optimization", "web"],
  ["mobile", "ios", "android"],
  ["design", "ux", "ui"],
  ["agile", "management", "process"],
  ["career", "advice", "development"],
  ["startup", "entrepreneurship", "business"],
  ["opensource", "community", "contribution"],
  ["architecture", "scalability", "design"],
];

const TITLE_TEMPLATES = [
  (topic) => `Getting Started with ${topic}: A Complete Guide`,
  (topic) => `Advanced ${topic} Techniques You Need to Know`,
  (topic) => `Why ${topic} is Changing the Industry`,
  (topic) => `Building Scalable Applications with ${topic}`,
  (topic) => `The Future of ${topic}: Trends and Predictions`,
  (topic) => `Common ${topic} Mistakes and How to Avoid Them`,
  (topic) => `Best Practices for ${topic} Development`,
  (topic) => `From Beginner to Expert in ${topic}`,
  (topic) => `Case Study: Implementing ${topic} in Production`,
  (topic) => `${topic} Tools and Frameworks Comparison`,
];

const CONTENT_TEMPLATES = [
  (
    topic,
  ) => `In this comprehensive guide, we'll explore the fundamentals of ${topic} and how it's revolutionizing the way we approach modern development. Whether you're a seasoned developer or just starting out, this article will provide valuable insights into best practices and real-world applications.

## Introduction to ${topic}

${topic} has become an essential part of today's technology landscape. With its powerful capabilities and growing ecosystem, it offers developers unprecedented opportunities to build robust and scalable solutions.

## Key Concepts

Understanding the core concepts of ${topic} is crucial for success. Let's dive into the fundamental principles that make this technology so powerful:

1. **Core Architecture**: The foundation of ${topic} lies in its well-designed architecture
2. **Performance Optimization**: Learn how to maximize efficiency and minimize resource usage
3. **Integration Patterns**: Discover best practices for connecting with other systems
4. **Security Considerations**: Implement robust security measures from the ground up

## Real-World Applications

Many companies have successfully implemented ${topic} in their production environments. Here are some notable examples:

- **Case Study 1**: A major e-commerce platform reduced their response time by 60%
- **Case Study 2**: A fintech startup improved their scalability by 300%
- **Case Study 3**: A healthcare company enhanced their data processing capabilities

## Getting Started

Ready to dive in? Here's a step-by-step guide to get you started with ${topic}:

\`\`\`javascript
// Example implementation
const example = new ${topic}();
example.initialize();
example.process();
\`\`\`

## Conclusion

${topic} represents a significant advancement in technology, offering developers powerful tools to build the next generation of applications. By following the principles and practices outlined in this guide, you'll be well-equipped to leverage ${topic} in your own projects.

Remember, the key to success with ${topic} is continuous learning and experimentation. Stay curious, keep building, and don't hesitate to explore new possibilities!`,

  (
    topic,
  ) => `The landscape of ${topic} is constantly evolving, and staying ahead of the curve requires a deep understanding of both current trends and emerging technologies. In this article, we'll examine the latest developments and provide actionable insights for developers looking to enhance their skills.

## Current State of ${topic}

Today's ${topic} ecosystem is more mature and feature-rich than ever before. With improved tooling, better documentation, and a growing community, developers have access to resources that make implementation more straightforward.

## Emerging Trends

Several key trends are shaping the future of ${topic}:

- **Automation**: Increasing focus on automated workflows and CI/CD integration
- **Performance**: New optimization techniques that improve speed and efficiency
- **Security**: Enhanced security features and best practices
- **Scalability**: Better support for large-scale deployments

## Industry Impact

The adoption of ${topic} across various industries has been remarkable:

- **Technology Sector**: 85% of tech companies have implemented ${topic} solutions
- **Financial Services**: Improved transaction processing and risk management
- **Healthcare**: Enhanced patient data management and analysis
- **E-commerce**: Better customer experience and operational efficiency

## Implementation Strategies

When implementing ${topic}, consider these strategic approaches:

1. **Phased Rollout**: Start with pilot projects before full deployment
2. **Team Training**: Invest in comprehensive team education
3. **Monitoring**: Implement robust monitoring and alerting systems
4. **Documentation**: Maintain detailed documentation for future reference

## Future Outlook

Looking ahead, ${topic} is poised for continued growth and innovation. Key areas to watch include:

- Advanced AI integration
- Improved developer experience
- Enhanced security features
- Better cross-platform compatibility

The future of ${topic} is bright, and developers who invest in learning these technologies now will be well-positioned for success in the years to come.`,

  (
    topic,
  ) => `Building robust applications with ${topic} requires more than just technical knowledge—it demands a strategic approach to architecture, design, and implementation. In this deep dive, we'll explore advanced techniques that will elevate your ${topic} development skills.

## Architecture Patterns

Effective ${topic} applications rely on well-established architectural patterns:

### Microservices Architecture
Breaking down monolithic applications into smaller, manageable services provides better scalability and maintainability.

### Event-Driven Design
Implementing event-driven patterns enables better decoupling and improved system responsiveness.

### Domain-Driven Design
Organizing code around business domains leads to more maintainable and understandable applications.

## Performance Optimization

Optimizing ${topic} applications requires attention to multiple factors:

- **Caching Strategies**: Implement intelligent caching to reduce database load
- **Resource Management**: Optimize memory usage and CPU utilization
- **Network Optimization**: Minimize network overhead and latency
- **Database Tuning**: Optimize queries and indexing strategies

## Testing Strategies

Comprehensive testing is essential for reliable ${topic} applications:

\`\`\`javascript
// Example test structure
describe('${topic} Component', () => {
  it('should handle basic functionality', () => {
    const component = new ${topic}Component();
    expect(component.process()).toBeDefined();
  });
  
  it('should handle edge cases', () => {
    const component = new ${topic}Component();
    expect(() => component.process(null)).not.toThrow();
  });
});
\`\`\`

## Monitoring and Observability

Implementing comprehensive monitoring helps identify issues before they impact users:

- **Application Metrics**: Track performance indicators and user behavior
- **Error Tracking**: Monitor and alert on application errors
- **Log Analysis**: Centralize and analyze application logs
- **Health Checks**: Implement automated health monitoring

## Security Considerations

Security should be a primary concern when developing ${topic} applications:

1. **Input Validation**: Always validate and sanitize user inputs
2. **Authentication**: Implement robust authentication mechanisms
3. **Authorization**: Control access to resources and functionality
4. **Data Protection**: Encrypt sensitive data both in transit and at rest

## Deployment Strategies

Successful deployment requires careful planning and execution:

- **Blue-Green Deployment**: Minimize downtime during updates
- **Canary Releases**: Gradually roll out changes to a subset of users
- **Feature Flags**: Control feature availability without code changes
- **Rollback Procedures**: Prepare for quick rollback in case of issues

## Conclusion

Mastering ${topic} development is an ongoing journey that requires continuous learning and adaptation. By implementing these advanced techniques and best practices, you'll build more robust, scalable, and maintainable applications.

The key to success lies in understanding not just the technical aspects, but also the business context and user needs. Keep experimenting, stay updated with the latest developments, and always prioritize code quality and user experience.`,
];

function pick(arr) {
  return arr[Math.floor(Math.random() * arr.length)];
}

export function generateBlogPosts(count = 100) {
  const posts = [];
  const now = new Date();
  const sixMonthsAgo = new Date(now.getTime() - 6 * 30 * 24 * 60 * 60 * 1000);

  for (let i = 1; i <= count; i++) {
    const topic = pick(TOPICS);
    const randomTime =
      sixMonthsAgo.getTime() +
      Math.random() * (now.getTime() - sixMonthsAgo.getTime());

    posts.push({
      title: pick(TITLE_TEMPLATES)(topic),
      content: pick(CONTENT_TEMPLATES)(topic),
      author: pick(AUTHORS),
      publish_date: new Date(randomTime).toISOString().split("T")[0],
      tags: pick(TAG_SETS),
    });
  }
  return posts;
}
